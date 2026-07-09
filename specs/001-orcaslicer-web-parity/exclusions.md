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
| `bbl_*`, `print_sequence` liés au cloud BBL (`preset_names` physiques…) | à trancher à l'implémentation | Chaque clé restante sera classée ici lors du codegen (aucun droit à l'oubli : le contrôle échoue tant qu'une clé n'est ni implémentée ni listée) |

## Interface (Annexe B)

| Entrée(s) | Statut | Justification |
|---|---|---|
| Menus « Upload Models » / « Download Models » (publish) | exclu-définitif | Service cloud MakerWorld/BBL, hors périmètre auto-hébergé |
| « Show/Hide 3Dconnexion settings » (Ctrl+M) | exclu-définitif | Périphérique local inaccessible depuis un navigateur |
| « Open Network Test », « Show Configuration Folder », « New Window » | exclu-définitif | Notions de bureau (fenêtres, dossiers locaux) sans objet web ; équivalents : diagnostic Moonraker, export de presets |
| Menu Calibration (8 items) + assistants CalibrationWizard | exclu-v1 | Backlog v2 exhaustif dans spec.md (décision utilisateur) |
| Raccourcis en conflit navigateur (ex. Ctrl+W, F5 si présents) | adaptation | Remappés, documentés dans l'aide raccourcis in-app ; liste finale complétée à l'implémentation |
| Écrans « Device »/monitor Bambu (AMS, caméra) | exclu-définitif | Matériel propriétaire ; supervision assurée via Moonraker (US8) |

## Presets (Annexe C)

| Entrée(s) | Statut | Justification |
|---|---|---|
| — | — | Aucune exclusion : les 11 895 presets des 65 vendeurs sont importés (FR-020) |

## Règles de tenue du registre

1. Une entrée « à trancher à l'implémentation » doit être résolue avant la
   fin du jalon qui touche son périmètre — le contrôle la compte comme écart.
2. Chaque `exclu-v1` doit pointer vers une entrée du backlog v2 de spec.md.
3. Ce fichier est modifié par PR revue, jamais en même temps qu'on « fait
   passer » le contrôle silencieusement.
