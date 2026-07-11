# Registre d'exclusions de parité (constitution V)

Toute entrée des Annexes A/B/C non implémentée est consignée ici avec
justification. `audit/check_traceability.py` échoue sur tout écart absent de
ce registre. Statuts : `exclu-v1` (backlog v2 tracé dans spec.md) ou
`exclu-définitif` (sans objet en web, justifié).

## Paramètres (Annexe A)

| Entrée(s) | Statut | Justification |
|---|---|---|
| Groupe `sla` (76 clés) | exclu-v1 (UI) | OrcaSlicer n'expose pas d'onglet SLA ; parité de données seulement (FR-004) — stockées/validées, pas d'UI |
| `host_type` valeurs ≠ `moonraker` (15) | exclu-v1 (intégration) | Clarification 2026-07-09 : Moonraker seul en v1 ; enum complet conservé en données ; backlog v2 spec.md |
| Groupes `other:*` (55 clés) | implémenté hors UI | Placeholders G-code / états de slicing : consommés par le moteur de templates, jamais montrés comme réglages (comportement identique à OrcaSlicer) |
| Groupes `cli:*` (52 clés) | implémenté hors UI | Actions CLI : correspondent aux endpoints serveur (slice, arrange, orient…), pas à des réglages UI (comportement identique à OrcaSlicer) |
| `printhost_*` (apikey, cafile, port, user, password, authorization_type, ssl_ignore_revoke) | partiel-v1 | Déclaration d'imprimante v1 = URL + clé API Moonraker ; champs restants activés avec les hôtes v2 |
| `bbl_calib_mark_logo`, `bbl_use_printhost`, `print_sequence`, `first_layer_print_sequence`, `other_layers_print_sequence`, `other_layers_print_sequence_nums` | implémenté | Réglages FFF/`common` standards : présents au registre (P1, 858 clés) et dans le layout de réglages (P3). Les noms `bbl_*` sont d'origine Bambu mais restent des bascules de réglage ordinaires (données + UI), sans dépendance cloud |

## Interface (Annexe B)

| Entrée(s) | Statut | Justification |
|---|---|---|
| Menus `Upload Models` / `Download Models` (publish) | exclu-définitif | Service cloud MakerWorld/BBL, hors périmètre auto-hébergé |
| Menu Aide : `Setup Wizard`, `Troubleshoot Center`, `Check for Updates`, `Show Tip of the Day` | exclu-définitif | Assistant de première configuration, diagnostic et mise à jour propres au bureau ; en web l'app est servie à jour et l'onboarding diffère |
| `Show Configuration Folder`, `New Window`, `Reset Window Layout` | exclu-définitif | Notions de bureau (fenêtres, dossiers locaux) sans objet web |
| `Open Network Test`, `Troubleshoot Center` | exclu-définitif | Diagnostics réseau/services Bambu propriétaires ; équivalent web : test de connexion Moonraker (US8) |
| Fichier : `Quit`, `&Quit`, `Open &Slicer…` | exclu-définitif | Quitter/lancer une autre instance = notions d'application de bureau (l'onglet du navigateur en tient lieu) |
| Fichier : `Sync Presets` | exclu-v1 | Synchronisation cloud BBL des presets ; le système de presets web est autonome (import/export de bundle assurés) |
| Vue : `Show 3D Navigator` | exclu-v1 | Cube de navigation 3Dconnexion/overlay ; backlog v2 (orbite caméra couverte par les vues Ctrl+0–6) |
| `Show/Hide 3Dconnexion settings` — raccourcis `Ctrl+M`, `Ctrl+Shift+M` | exclu-définitif | Périphérique local inaccessible depuis un navigateur |
| Raccourci `Ctrl+Tab` (Switch table page) | adaptation | Réservé par le navigateur (changement d'onglet) ; remappé in-app, documenté dans l'aide raccourcis |
| Raccourcis réservés par le navigateur : `Ctrl+N` (New Project), `F5` (Reload from Disk) | adaptation | Déclenchés par l'item de menu correspondant (menus.ts) qui reste fonctionnel ; l'accélérateur brut peut être capté par le navigateur (nouvelle fenêtre / rafraîchissement). Les autres accélérateurs des 92 raccourcis sont interceptables (`preventDefault`) et actifs. Liste close (§B.6 revue) |
| Menu Calibration (`Temperature`, `Max flowrate`, `Pressure advance`, `Flow ratio`, `Retraction`, `Cornering`, `VFA`, `Calibration Guide`, `Input Shaping Frequency`, `Input Shaping Damping/zeta factor`) + assistants CalibrationWizard | exclu-v1 | Backlog v2 exhaustif dans spec.md (décision utilisateur) |
| Écrans « Device »/monitor Bambu (AMS, caméra) | exclu-définitif | Matériel propriétaire ; supervision assurée via Moonraker (US8) |

## Presets (Annexe C)

| Entrée(s) | Statut | Justification |
|---|---|---|
| — | — | Aucune exclusion : les 11 895 presets des 65 vendeurs sont importés (FR-020) |

## Comportement moteur (slicing FFI)

| Entrée(s) | Statut | Justification |
|---|---|---|
| Vignettes PNG embarquées lors du tranchage FFI (`SliceResult.thumbnails`) | exclu-v1 | La génération des vignettes passe par le rendu OpenGL de la GUI (`ThumbnailsGeneratorCallback`) ; la lib `libslic3r-headless` n'embarque pas de contexte GL. Le tranchage renvoie G-code + stats ; les vignettes de préviz seront rendues côté frontend (Three.js) ou par un renderer offscreen dédié en v2 (backlog spec.md) |

## Règles de tenue du registre

1. Une entrée « à trancher à l'implémentation » doit être résolue avant la
   fin du jalon qui touche son périmètre — le contrôle la compte comme écart.
2. Chaque `exclu-v1` doit pointer vers une entrée du backlog v2 de spec.md.
3. Ce fichier est modifié par PR revue, jamais en même temps qu'on « fait
   passer » le contrôle silencieusement.
