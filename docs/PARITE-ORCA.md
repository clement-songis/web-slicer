# Parité OrcaSlicer 2.4.1 → web-slicer

État au 2026-11-14, consolidé depuis l'inventaire de parité
(`specs/001-orcaslicer-web-parity/annexes/`, `audit/ui_inventory.json`), l'état du
harness (`harness/features.json`, faisant foi : `$HARNESS_STATE/features.json`) et les
rapports des sessions de consolidation.

Légende : **fait** = implémenté et prouvé par un test automatique ; **partiel** =
implémenté avec des écarts connus, ou sans preuve automatique complète ; **absent** = non
implémenté. Statuts harness : `passing` (validé par `verify.py`), `red` (test écrit, pas
encore promu), `needs_test` (scénario à écrire), `needs_ref`, `needs_review`, `excluded`.

## Synthèse

| Domaine | Mesure |
|---|---|
| Moteur (G-code) | Corpus de parité rejoué le 2026-11-14 contre le moteur actuel : **626/626** G-code identiques à Orca (hors commentaires) + **7/7** refus Orca refusés aussi |
| Golden (6 modèles × 3 process) | **18/18** verts en `verify.py --check` après le correctif d'arrangement (cylindres compris) ; l'état harness affiche encore 15 `passing` + 3 `needs_ref` (cylindres) tant que `verify.py` n'a pas réécrit l'état |
| Paramètres (registre Orca) | 858 : 409 `passing`, 199 `needs_ref`, 67 `needs_review`, 183 `excluded` (SLA, CLI, statistiques sans UI) |
| Parcours core | 8/9 `passing` ; `core.moonraker` `red` |
| Éditeur (e2e ajoutés) | 33 features `editor.*` déclarées (`red`, test écrit) — à promouvoir par `verify.py` après regel |
| Menus / contextuel / raccourcis / gizmos / barre d'outils (scénarios générés) | 79 + 64 + 90 + 16 + 10 en `needs_test` : scénarios gelés non écrits. Une partie est couverte par les e2e `editor.*` |

## Tranchage et G-code

| Fonctionnalité Orca | État | Preuve | Limites |
|---|---|---|---|
| Tranchage libslic3r (même moteur qu'Orca 2.4.1, FFI) | fait | `core.harness-api`, golden 18/18, corpus 626/626 | — |
| Paramètres process / filament / imprimante | fait | `params.spec.ts` (« moteur ») : 626/626 | 67 `needs_review` (valeur de test refusée par Orca ou champ désactivé), à arbitrer |
| Validation de config du CLI (`validate(true)` : bornes, incompatibilités) | fait | 7/7 refus Orca reproduits (`/var/tmp/probe_refusals.py`) | 2 refus sortent un message interne libslic3r, pas le message générique d'Orca |
| Arrangement (`--arrange 1`, rotations autorisées) | fait | `engine/tests/ffi_arrange_angle.rs` (angle −137,812452° = CLI), golden cylindre | Inflation 1 mm par défaut alignée sur Orca ; réplique de diagnostic `engine/tests/replica/` |
| Déterminisme (gap fill 1re couche, refs instables) | fait | `tests/oracle/refs/REGENERATED.md` (3 tirages Orca par ref) | Refs régénérées : regel nécessaire |
| Calibrations Orca (température, PA, débit, rétraction, VFA, débit volumétrique max) | partiel | `editor.calibration-temp`, `editor.calibration-retraction`, tests vitest `lib/calibration` | Seules température et rétraction ont un e2e ; flowrate / PA / VFA / MVS testés en unitaire |
| Hauteur de couche variable | partiel | vitest `layer-height.test.ts` | `ui.height-range-modifier` en `needs_test` |

## Scène, objets, volumes

| Fonctionnalité Orca | État | Preuve | Limites |
|---|---|---|---|
| Import STL / OBJ / 3MF / STEP | fait | `core.import`, tests backend `models` | — |
| Liste d'objets (verrou, masquage, extrudeur, groupe, duplication) | fait | `editor-scene.spec.ts` | — |
| Renommer objet / plateau | fait | `editor.rename` | — |
| Annuler / Rétablir | fait | `editor.undo-redo` | — |
| Gizmos déplacer / tourner / échelle / poser à plat | partiel | panneau de transformation, `editor-volume-transform` (volumes) | `gizmo.*` en `needs_test` |
| Réglages par objet (Add settings) | fait | `editor.object-settings`, `editor.object-panel` | Listes de clés = `SettingsFactory::get_options` (région + objet) |
| Réglages par plage de hauteur | absent | — | `ui.height-range-modifier` |
| Volumes : pièce, négatif, modificateur, bloqueur, forceur | fait | `editor.volume-negative`, `editor.volume-modifier`, `editor.volume-blocker` | Primitive pavé uniquement (pas d'import de maillage en volume) |
| Volumes : rendu 3D, sélection au clic, transformation | partiel | `editor.volume-transform` | Pas de gizmo 3D pour un volume ; clic 3D sans test automatique ; rotation au tranchage active après redémarrage du backend |
| Peinture supports / couture | fait | `editor.paint-supports`, `editor.paint-supports-local`, `editor.paint-seam` | Pinceau non pilotable en e2e (facettes injectées) |
| Peinture multi-matériau, fuzzy skin | partiel | — | `gizmo.mmsegmentation`, `gizmo.fuzzyskin` en `needs_test` |
| Outils maillage : réparation, simplification, séparation, coupe, booléen | fait | `editor.mesh-repair`, `editor.mesh-simplify`, `editor.mesh-simplify-error`, `editor.mesh-split`, `editor.mesh-cut`, `editor.mesh-boolean` | — |
| Emboss texte | fait | `editor.emboss-text` | SVG (`gizmo.svg`) non couvert |
| Mesure, assemblage, oreilles de bordure | partiel | composants `MeasureTool`, `AssemblyView`, `BrimEarsTool` | Sans e2e (`gizmo.measure`, `gizmo.assembly`, `gizmo.brimears`) |

## Plateaux et multi-filament

| Fonctionnalité Orca | État | Preuve | Limites |
|---|---|---|---|
| Multi-plateaux (ajout, suppression, affectation, duplication) | fait | `editor.plate-duplicate`, `editor.preview-plates` | — |
| Réglages par plateau (type de plaque, séquence, vase spirale) | fait | `editor.plate-settings` (G-code), test backend `plate_settings_override_the_snapshot_for_their_plate_only` | Ordre d'extrudeurs 1re couche / autres couches non exposé dans l'UI ; le libellé `plateType` seul ne change pas le tranchage (seul un choix explicite compte) |
| Slots filament (ajout / retrait, couleurs) | fait | `editor.filament-slots` | — |
| Export de tous les plateaux | fait | `editor.menu-export-plates` | — |

## Aperçu, export, menus

| Fonctionnalité Orca | État | Preuve | Limites |
|---|---|---|---|
| Aperçu G-code (couches, légende, coloration filament) | fait | `core.preview`, `editor.preview-plates` | — |
| Fenêtre G-code synchronisée | fait | `editor.view-gcode-window` | — |
| Export des trajets en OBJ | fait | `editor.export-toolpaths-obj` | — |
| Ouvrir un G-code local (visionneuse) | fait | `editor.menu-open-gcode` | — |
| Export STL (fichier / archive par objet) | fait | `editor.menu-export-stl` | — |
| Export / import 3MF (peinture, volumes, réglages) | fait | `editor.export-3mf-paint-volumes`, `editor.import-3mf-paint-volumes` | Brièvement passé en `test.fixme` par une session, réactivé depuis ; à reconfirmer par `verify.py` après regel |
| Vue : orthogonale, étiquettes, surplombs, préférences | fait | `editor.menu-view-prefs`, `editor.view-labels-overhang` | — |
| Menus principaux (Fichier, Édition, Vue…) | partiel | e2e `editor-menus.spec.ts` | 79 entrées `menu.*` en `needs_test`, 15 `excluded` (cloud Bambu / bureau) |
| Menu contextuel objet | partiel | e2e `editor-object-*` (Add settings, ajout de volumes) | 64 `ctx.*` en `needs_test` ; « Renommer » du menu contextuel reste un `menuTodo` (le renommage passe par la liste d'objets) |
| Raccourcis clavier | partiel | — | 90 `shortcut.*` en `needs_test` |

## Imprimantes et production

| Fonctionnalité Orca | État | Preuve | Limites |
|---|---|---|---|
| Envoi Moonraker | partiel | page `/printers`, file d'impression | `core.moonraker` `red` : e2e vert non obtenu (les rejeux ont tourné avec `HARNESS_MOONRAKER=0`) |
| Presets (système, utilisateur, « Enregistrer sous ») | partiel | `core.settings-ui` | `ui.preset-save-as` en `needs_test` |
| Déploiement de production (adapter-node + proxy `/api`) | fait | Build isolé `/var/tmp/fe-prod` + `node server.js` : POST multipart `/api/_harness/slice` → 200, formulaire cross-origin sur une route SvelteKit → 403 ; vitest `hooks.server.test.ts` | Définir `ORIGIN` en production (sinon une route SvelteKit en même origine est aussi refusée, comme le CSRF natif de SvelteKit) ; `CSRF_TRUSTED_ORIGINS` optionnel |
| Services cloud Bambu, bureau | exclu | `harness/exclusions.json` | Sans objet en web |

## Étapes restantes pour clore la parité

1. Regeler (`python3 harness/loopctl.py freeze`, cf. `harness/README.md` § Gel), puis
   `verify.py` sur les 33 `editor.*`, les golden cylindres et les paramètres rejoués,
   pour aligner l'état harness sur les mesures ci-dessus.
2. Arbitrer les 67 paramètres `needs_review` et produire les 199 refs `needs_ref`.
3. Écrire les scénarios gelés `gizmo` / `toolbar` / `menu` / `ctx` / `shortcut` restants
   (session « tests »), en réutilisant les e2e `editor.*` déjà verts.
4. Moonraker : rejouer `core.moonraker` avec un Moonraker de test actif.
