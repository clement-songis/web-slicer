# Audit OrcaSlicer

Scripts d'inventaire qui parsent `vendor/OrcaSlicer` et génèrent des JSON
de référence. Ré-exécutables à tout moment — ils servent de contrôle final
pour vérifier la couverture du web-slicer par rapport à OrcaSlicer.

## Usage

```sh
python3 audit/run_all.py        # régénère tout + contrôles croisés
python3 audit/extract_parameters.py         # parameters.json seul
python3 audit/extract_ui_inventory.py       # ui_inventory.json seul
python3 audit/extract_engine_api.py         # engine_api.json seul
python3 audit/extract_presets_inventory.py  # presets_inventory.json seul
```

Aucune dépendance hors bibliothèque standard (Python ≥ 3.10).

## Sorties

| Fichier | Contenu | Source parsée |
|---|---|---|
| `parameters.json` | Tous les `ConfigOptionDef` : clé, type, défaut, min/max, tooltip, catégorie, mode simple/advanced/expert/develop, enums | `src/libslic3r/PrintConfig.cpp` (+ `PrintConfigConstants.hpp` pour résoudre les macros) |
| `ui_inventory.json` | Onglets de réglages (pages → sections → options), menus principaux et contextuels, barres d'outils 3D, gizmos, raccourcis clavier | `src/slic3r/GUI/Tab.cpp`, `MainFrame.cpp`, `GUI_Factories.cpp`, `GLCanvas3D.cpp`, `Gizmos/GLGizmosManager.cpp`, `KBShortcutsDialog.cpp` |
| `engine_api.json` | Classes et méthodes publiques du pipeline de slicing (Model, Print, PrintObject, Slicing, TriangleMesh, Preset(Bundle), GCode…), annotées avec leur usage par la GUI | headers de `src/libslic3r/` (liste `HEADERS` dans le script) |
| `presets_inventory.json` | Structure des profils système : vendeurs, presets machine/filament/process, héritage (`inherits`), fréquence des clés de config par type | `resources/profiles/` |

## Notes de fiabilité

- Les extracteurs sont des parsers par regex/scan, pas un frontend C++ :
  quelques défauts complexes restent en texte brut (`default_raw`).
- Dans `parameters.json`, les clés `filament_*` de surcharge extrudeur sont
  générées dynamiquement dans PrintConfig.cpp ; elles sont reconstruites à
  l'identique (champ `derived_from`).
- Dans `engine_api.json`, `gui_call_sites` compte les occurrences
  `.méthode(` / `->méthode(` / `::méthode(` dans `src/slic3r` — les homonymes
  (ex. `clear`) sont donc comptés tous receveurs confondus.
- `run_all.py` vérifie que chaque option affichée dans les onglets existe dans
  `parameters.json`, et liste les clés de presets inconnues de
  PrintConfig.cpp (clés legacy gérées par `handle_legacy`, clés spécifiques
  aux forks vendeurs, fautes de frappe historiques).
