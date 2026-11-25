# Annexe C — Inventaire des profils système (normatif)

Source : `audit/presets_inventory.json`. La liste nominative complète des
14735 presets est portée par ce fichier JSON,
incorporé par référence : chaque preset qu'il liste DOIT être importé et
utilisable (héritage résolu) — le contrôle se fait par comptage exact
type par type via `audit/run_all.py`, pas par relecture manuelle.

## C.1 Totaux normatifs

- Presets : 14735
- Par type : {"filament": 9034, "machine": 1474, "machine_model": 455, "process": 3772}
- Instanciables (visibles utilisateur) : {"filament": 7637, "machine": 1194, "process": 3259}

## C.2 Vendeurs (chaque ligne est une exigence d'import)

| Vendeur | Version | machine_model | machine | filament | process |
|---|---|---|---|---|---|
| Afinia | 02.04.00.06 | 1 | 4 | 10 | 24 |
| Anker | 02.04.00.08 | 3 | 14 | 61 | 26 |
| Anycubic | 02.04.00.10 | 21 | 33 | 183 | 114 |
| Artillery | 02.04.00.06 | 10 | 14 | 67 | 67 |
| BBL | 02.08.00.16 | 14 | 148 | 2050 | 278 |
| BIQU | 02.04.00.04 | 3 | 6 | 0 | 15 |
| BenFab | 01.00.00.01 | 1 | 6 | 0 | 24 |
| Blocks | 02.04.00.15 | 3 | 13 | 45 | 48 |
| CONSTRUCT3D | 02.04.00.04 | 2 | 3 | 6 | 9 |
| Chuanying | 02.04.00.07 | 1 | 8 | 17 | 8 |
| Co Print | 02.04.00.04 | 1 | 6 | 6 | 4 |
| CoLiDo | 02.04.00.08 | 5 | 7 | 18 | 34 |
| Comgrow | 02.04.00.06 | 2 | 6 | 8 | 18 |
| Creality | 02.03.02.90 | 57 | 122 | 772 | 416 |
| Cubicon | 02.04.00.08 | 3 | 4 | 42 | 7 |
| Custom | 02.04.00.09 | 6 | 21 | 10 | 41 |
| DeltaMaker | 02.04.00.04 | 3 | 5 | 8 | 5 |
| Dremel | 02.04.00.04 | 3 | 5 | 5 | 15 |
| Elegoo | 02.04.00.14 | 18 | 76 | 268 | 271 |
| Eryone | 02.04.00.07 | 3 | 15 | 46 | 100 |
| FELIX Printers | 01.00.01.02 | 7 | 40 | 107 | 40 |
| FLSun | 02.04.00.05 | 6 | 7 | 33 | 27 |
| Flashforge | 02.04.00.14 | 13 | 55 | 607 | 134 |
| FlyingBear | 02.04.00.07 | 4 | 7 | 59 | 24 |
| Folgertech | 02.04.00.03 | 3 | 8 | 0 | 15 |
| Geeetech | 02.04.00.02 | 16 | 46 | 0 | 89 |
| Ginger Additive | 02.04.00.05 | 1 | 6 | 3 | 6 |
| Goofoo | 02.04.00.02 | 4 | 8 | 0 | 7 |
| IdeaFormer | 02.00.00.07 | 1 | 4 | 3 | 2 |
| InfiMech | 02.04.00.07 | 4 | 6 | 88 | 24 |
| Kingroon | 02.04.00.04 | 5 | 7 | 0 | 9 |
| LH | 02.04.00.07 | 2 | 5 | 15 | 14 |
| LONGER | 02.04.00.05 | 2 | 9 | 7 | 49 |
| Lulzbot | 02.04.00.06 | 5 | 7 | 6 | 11 |
| M3D | 02.04.00.06 | 2 | 3 | 2 | 11 |
| MagicMaker | 02.04.00.07 | 5 | 6 | 2 | 27 |
| Mellow | 02.04.00.02 | 1 | 6 | 0 | 11 |
| Meltingplot | 02.04.00.02 | 5 | 28 | 40 | 32 |
| OpenEYE | 02.04.00.04 | 1 | 6 | 0 | 39 |
| OrcaArena | 02.04.00.09 | 1 | 6 | 96 | 47 |
| OrcaFilamentLibrary | 02.04.00.26 | 0 | 0 | 1441 | 0 |
| Peopoly | 02.04.00.04 | 1 | 5 | 11 | 19 |
| Phrozen | 02.04.00.06 | 1 | 2 | 10 | 2 |
| Positron3D | 02.04.00.02 | 1 | 6 | 0 | 11 |
| Pragostroj | 01.00.01.01 | 2 | 17 | 11 | 15 |
| Printcepts | 01.00.00.05 | 1 | 4 | 3 | 2 |
| Prusa | 02.04.00.20 | 18 | 76 | 363 | 373 |
| Qidi | 02.04.00.21 | 13 | 48 | 1782 | 242 |
| RH3D | 02.04.00.06 | 2 | 13 | 19 | 31 |
| Raise3D | 02.04.00.03 | 2 | 7 | 0 | 7 |
| Ratrig | 02.04.00.07 | 21 | 69 | 20 | 46 |
| RolohaunDesign | 02.04.00.03 | 2 | 7 | 0 | 18 |
| SecKit | 02.04.00.05 | 2 | 4 | 19 | 9 |
| SeeMeCNC | 2.4.0.07 | 7 | 28 | 7 | 132 |
| Snapmaker | 02.04.00.22 | 19 | 101 | 357 | 134 |
| Sovol | 02.04.00.10 | 13 | 34 | 32 | 53 |
| Tiertime | 02.04.00.05 | 4 | 10 | 95 | 65 |
| Tronxy | 02.04.00.02 | 1 | 2 | 0 | 8 |
| TwoTrees | 02.04.00.05 | 2 | 4 | 2 | 16 |
| UltiMaker | 02.04.00.06 | 4 | 12 | 27 | 18 |
| Vivedino | 02.04.00.14 | 6 | 18 | 0 | 28 |
| Volumic | 02.04.00.06 | 21 | 30 | 37 | 81 |
| Voron | 02.04.00.03 | 8 | 66 | 0 | 50 |
| Voxelab | 02.04.00.02 | 1 | 2 | 0 | 3 |
| Vzbot | 02.04.00.05 | 2 | 8 | 19 | 24 |
| WEMAKE3D | 02.04.00.03 | 2 | 11 | 0 | 41 |
| Wanhao | 02.04.00.04 | 13 | 17 | 0 | 14 |
| Wanhao France | 02.04.00.07 | 18 | 19 | 13 | 16 |
| WonderMaker | 02.04.00.05 | 3 | 15 | 34 | 65 |
| Z-Bolt | 02.04.00.05 | 9 | 29 | 20 | 85 |
| iQ | 02.04.00.05 | 2 | 9 | 7 | 12 |
| re3D | 03.00.13 | 6 | 15 | 15 | 10 |

## C.3 Structure d'un preset (champs méta observés)

```json
{
  "filament": {
    "type": 9034,
    "name": 9034,
    "instantiation": 9034,
    "from": 9003,
    "inherits": 8937,
    "compatible_printers": 7854,
    "setting_id": 7637,
    "filament_id": 4002,
    "compatible_printers_condition": 289,
    "description": 237
  },
  "machine": {
    "type": 1474,
    "name": 1474,
    "instantiation": 1474,
    "from": 1367,
    "inherits": 1277,
    "nozzle_diameter": 1272,
    "printer_model": 1203,
    "setting_id": 1194,
    "printer_variant": 1171,
    "default_bed_type": 29,
    "description": 28,
    "bed_texture": 19,
    "machine_tech": 12,
    "model_id": 12,
    "bed_model": 7,
    "default_materials": 6,
    "family": 1
  },
  "machine_model": {
    "type": 455,
    "name": 455,
    "nozzle_diameter": 455,
    "machine_tech": 455,
    "family": 455,
    "default_materials": 455,
    "model_id": 452,
    "bed_model": 440,
    "bed_texture": 418,
    "hotend_model": 359,
    "default_bed_type": 54,
    "url": 38,
    "description": 7,
    "instantiation": 6
  },
  "process": {
    "type": 3772,
    "name": 3772,
    "instantiation": 3772,
    "from": 3738,
    "inherits": 3683,
    "setting_id": 3259,
    "compatible_printers": 2778,
    "compatible_printers_condition": 827,
    "description": 393
  }
}
```

Héritage : champ `inherits` (chaîne parent) ; presets abstraits :
`instantiation=false` ; presets utilisateur : mêmes champs, `from=user`.