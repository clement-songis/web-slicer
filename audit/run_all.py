#!/usr/bin/env python3
"""Régénère les 4 inventaires d'audit puis vérifie leur cohérence croisée.

Usage : python3 audit/run_all.py

Contrôles effectués après extraction :
  - chaque option affichée dans les onglets (ui_inventory) existe dans
    parameters.json ;
  - chaque clé de config utilisée par les presets système existe dans
    parameters.json (les clés inconnues sont listées : legacy/renommées).
"""

from __future__ import annotations

import json
import sys

import extract_engine_api
import extract_parameters
import extract_presets_inventory
import extract_ui_inventory
from common import AUDIT_DIR


def load(name: str) -> dict:
    return json.loads((AUDIT_DIR / name).read_text(encoding="utf-8"))


def cross_check() -> int:
    params = set(load("parameters.json")["parameters"])
    ui = load("ui_inventory.json")
    presets = load("presets_inventory.json")

    problems = 0

    ui_keys = {
        opt
        for page in ui["settings_tabs"]
        for section in page["sections"]
        for opt in section["options"]
        if isinstance(opt, str)
    }
    unknown_ui = sorted(ui_keys - params)
    if unknown_ui:
        problems += len(unknown_ui)
        print(f"⚠ {len(unknown_ui)} option(s) affichée(s) dans l'UI sans définition "
              f"dans PrintConfig.cpp : {unknown_ui}")
    else:
        print("✓ toutes les options des onglets existent dans parameters.json")

    preset_keys = {
        k for usage in presets["key_usage_by_type"].values() for k in usage
    }
    unknown_presets = sorted(preset_keys - params)
    if unknown_presets:
        print(f"ℹ {len(unknown_presets)} clé(s) de preset absente(s) de PrintConfig.cpp "
              f"(souvent des clés legacy gérées par handle_legacy) :")
        print("  " + ", ".join(unknown_presets))
    else:
        print("✓ toutes les clés des presets existent dans parameters.json")

    return problems


def main() -> int:
    print("== 1/4 parameters.json ==")
    extract_parameters.main()
    print("== 2/4 ui_inventory.json ==")
    extract_ui_inventory.main()
    print("== 3/4 engine_api.json ==")
    extract_engine_api.main()
    print("== 4/4 presets_inventory.json ==")
    extract_presets_inventory.main()
    print("== contrôles croisés ==")
    cross_check()
    return 0


if __name__ == "__main__":
    sys.exit(main())
