# Annexe C — Inventaire des profils système (normatif)

Source : `audit/presets_inventory.json`. La liste nominative complète des
11895 presets est portée par ce fichier JSON,
incorporé par référence : chaque preset qu'il liste DOIT être importé et
utilisable (héritage résolu) — le contrôle se fait par comptage exact
type par type via `audit/run_all.py`, pas par relecture manuelle.

## C.1 Totaux normatifs

- Presets : 11895
- Par type : {"filament": 7012, "machine": 1158, "machine_model": 384, "process": 3341}
- Instanciables (visibles utilisateur) : {"filament": 5913, "machine": 1001, "process": 2881}

## C.2 Vendeurs (chaque ligne est une exigence d'import)

| Vendeur | Version | machine_model | machine | filament | process |
|---|---|---|---|---|---|
| Afinia | 02.04.00.02 | 1 | 4 | 10 | 24 |
| Anker | 02.04.00.01 | 3 | 14 | 51 | 26 |
| Anycubic | 02.04.00.02 | 20 | 29 | 156 | 98 |
| Artillery | 02.04.00.02 | 10 | 14 | 67 | 67 |
| BBL | 02.01.00.19 | 12 | 51 | 2197 | 243 |
| BIQU | 02.04.00.01 | 3 | 6 | 0 | 15 |
| Blocks | 02.04.00.02 | 3 | 13 | 20 | 48 |
| CONSTRUCT3D | 02.04.00.01 | 2 | 3 | 6 | 9 |
| Chuanying | 02.04.00.02 | 1 | 8 | 17 | 8 |
| Co Print | 02.04.00.01 | 1 | 6 | 6 | 4 |
| CoLiDo | 02.04.00.02 | 5 | 7 | 18 | 34 |
| Comgrow | 02.04.00.03 | 2 | 6 | 8 | 18 |
| Creality | 02.03.02.75 | 41 | 102 | 552 | 363 |
| Cubicon | 02.04.00.02 | 3 | 4 | 42 | 7 |
| Custom | 02.04.00.01 | 5 | 16 | 10 | 39 |
| DeltaMaker | 02.04.00.01 | 3 | 5 | 8 | 5 |
| Dremel | 02.04.00.01 | 3 | 5 | 6 | 15 |
| Elegoo | 02.04.00.06 | 18 | 76 | 245 | 271 |
| Eryone | 02.04.00.02 | 3 | 15 | 33 | 100 |
| FLSun | 02.04.00.01 | 6 | 7 | 33 | 27 |
| Flashforge | 02.04.00.04 | 12 | 49 | 570 | 116 |
| FlyingBear | 02.04.00.02 | 4 | 7 | 59 | 24 |
| Folgertech | 02.04.00.01 | 3 | 8 | 0 | 15 |
| Geeetech | 02.04.00.01 | 16 | 46 | 0 | 89 |
| Ginger Additive | 02.04.00.02 | 1 | 6 | 3 | 6 |
| InfiMech | 02.04.00.02 | 4 | 6 | 88 | 24 |
| Kingroon | 02.04.00.02 | 5 | 7 | 0 | 9 |
| LH | 02.04.00.02 | 2 | 5 | 15 | 14 |
| LONGER | 02.04.00.01 | 2 | 9 | 7 | 49 |
| Lulzbot | 02.04.00.01 | 4 | 6 | 3 | 9 |
| M3D | 02.04.00.02 | 1 | 2 | 0 | 3 |
| MagicMaker | 02.04.00.02 | 5 | 6 | 2 | 27 |
| Mellow | 02.04.00.01 | 1 | 6 | 0 | 11 |
| OpenEYE | 02.04.00.01 | 1 | 6 | 0 | 39 |
| OrcaArena | 02.04.00.02 | 1 | 6 | 96 | 47 |
| OrcaFilamentLibrary | 02.04.00.03 | 0 | 0 | 482 | 0 |
| Peopoly | 02.04.00.01 | 1 | 5 | 11 | 19 |
| Phrozen | 02.04.00.02 | 1 | 2 | 10 | 2 |
| Positron3D | 02.04.00.01 | 1 | 6 | 0 | 11 |
| Prusa | 02.04.00.03 | 13 | 64 | 272 | 334 |
| Qidi | 02.04.00.06 | 11 | 40 | 1287 | 209 |
| RH3D | 02.04.00.01 | 1 | 7 | 13 | 9 |
| Raise3D | 02.04.00.01 | 2 | 7 | 0 | 7 |
| Ratrig | 02.04.00.02 | 21 | 69 | 29 | 45 |
| RolohaunDesign | 02.04.00.01 | 2 | 7 | 0 | 18 |
| SecKit | 02.04.00.01 | 2 | 4 | 19 | 9 |
| SeeMeCNC | 2.4.0.02 | 7 | 28 | 7 | 132 |
| Snapmaker | 02.04.00.07 | 19 | 101 | 277 | 120 |
| Sovol | 02.04.00.01 | 13 | 23 | 31 | 35 |
| Tiertime | 02.04.00.01 | 4 | 10 | 95 | 65 |
| Tronxy | 02.04.00.01 | 1 | 2 | 0 | 8 |
| TwoTrees | 02.04.00.01 | 2 | 4 | 2 | 16 |
| UltiMaker | 02.04.00.01 | 1 | 2 | 0 | 4 |
| Vivedino | 02.04.00.01 | 2 | 5 | 0 | 8 |
| Volumic | 02.04.00.01 | 21 | 30 | 37 | 81 |
| Voron | 02.04.00.02 | 8 | 66 | 0 | 50 |
| Voxelab | 02.04.00.01 | 1 | 2 | 0 | 3 |
| Vzbot | 02.04.00.01 | 2 | 8 | 19 | 24 |
| WEMAKE3D | 02.04.00.01 | 2 | 11 | 0 | 41 |
| Wanhao | 02.04.00.01 | 1 | 3 | 0 | 6 |
| Wanhao France | 02.04.00.02 | 18 | 19 | 13 | 16 |
| WonderMaker | 02.04.00.01 | 3 | 14 | 34 | 65 |
| Z-Bolt | 02.04.00.01 | 9 | 29 | 20 | 85 |
| iQ | 02.04.00.01 | 2 | 9 | 7 | 6 |
| re3D | 02.04.00.03 | 6 | 15 | 19 | 10 |

## C.3 Structure d'un preset (champs méta observés)

```json
{
  "filament": {
    "type": 7012,
    "name": 7012,
    "instantiation": 7012,
    "from": 6981,
    "inherits": 6929,
    "compatible_printers": 6020,
    "setting_id": 5913,
    "filament_id": 2742,
    "compatible_printers_condition": 246,
    "description": 196,
    "version": 27
  },
  "machine": {
    "type": 1158,
    "name": 1158,
    "instantiation": 1158,
    "from": 1140,
    "nozzle_diameter": 1077,
    "inherits": 1057,
    "printer_model": 1009,
    "setting_id": 1001,
    "printer_variant": 987,
    "description": 28,
    "bed_texture": 19,
    "default_bed_type": 18,
    "machine_tech": 12,
    "model_id": 12,
    "bed_model": 7,
    "default_materials": 6,
    "family": 1
  },
  "machine_model": {
    "type": 384,
    "name": 384,
    "nozzle_diameter": 384,
    "machine_tech": 384,
    "family": 384,
    "bed_model": 382,
    "model_id": 381,
    "default_materials": 361,
    "bed_texture": 359,
    "hotend_model": 309,
    "url": 36,
    "default_bed_type": 35,
    "description": 7,
    "instantiation": 6
  },
  "process": {
    "type": 3341,
    "name": 3341,
    "instantiation": 3341,
    "from": 3307,
    "inherits": 3259,
    "setting_id": 2881,
    "compatible_printers": 2413,
    "compatible_printers_condition": 778,
    "description": 363
  }
}
```

Héritage : champ `inherits` (chaîne parent) ; presets abstraits :
`instantiation=false` ; presets utilisateur : mêmes champs, `from=user`.