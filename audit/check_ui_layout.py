#!/usr/bin/env python3
"""Contrôle FIL-a (lot 47) : layout généré vs Tab.cpp d'OrcaSlicer.

1. liste les sections vides de tous les onglets (frontend/src/generated/ui-layout.ts) ;
2. liste les clés citées dans Tab.cpp (append_single_option_line("k"),
   get_option("k"), append_option_line(.., "k"), boucles littérales) qui
   n'apparaissent dans aucune section du layout ;
3. liste les marqueurs { dynamic } restants.
Sortie non nulle si une section vide ou une clé manquante n'est pas justifiée.
"""
from __future__ import annotations

import json
import re
import sys

from common import AUDIT_DIR, read_source, strip_comments

# Sections vides légitimes : optgroup sans option (légende du mode silencieux,
# TabPrinter::build_kinematics_page, Tab.cpp « new_optgroup("") » + widgets).
ALLOWED_EMPTY = {("machine", "Motion ability", "")}
# Clés citées par Tab.cpp hors des pages de réglages (widgets dédiés,
# panneaux non génériques) : rendues par des composants spécifiques.
ALLOWED_MISSING = {
    "printable_area",          # create_line_with_widget → dialogue Bed Shape
    "compatible_printers",     # create_line_with_widget → liste de cases
    "compatible_prints",       # idem
    "filament_ramming_parameters",  # create_line_with_widget → RammingDialog (WIDGET_LINES)
}


def main() -> int:
    inv = json.load(open(AUDIT_DIR / "ui_inventory.json", encoding="utf-8"))
    from generate_frontend_ts import page_kind, page_title

    present: set[str] = set()
    empty, dynamic = [], []
    for tab in inv["settings_tabs"]:
        kind, title = page_kind(tab.get("defined_in") or ""), page_title(tab)
        for s in tab["sections"]:
            opts = s["options"]
            if not opts and (kind, title, s["title"] or "") not in ALLOWED_EMPTY:
                empty.append(f"{kind}/{title}/{s['title']}")
            for o in opts:
                if isinstance(o, str):
                    present.add(o)
                else:
                    dynamic.append(f"{kind}/{title}/{s['title']}: {o}")

    code = strip_comments(read_source("src/slic3r/GUI/Tab.cpp"))
    cited = set(re.findall(r'append_single_option_line\s*\(\s*"([^"]+)"', code))
    cited |= set(re.findall(r'get_option\s*\(\s*"([^"]+)"', code))
    cited |= set(re.findall(r'append_option_line\s*\(\s*\w+\s*,\s*"([^"]+)"\s*[,)]', code))
    cited |= set(re.findall(r'create_line_with_widget\s*\(\s*[^,]+,\s*"([^"]+)"', code))
    params = set(json.load(open(AUDIT_DIR / "parameters.json", encoding="utf-8"))["parameters"])
    missing = sorted(k for k in cited if k in params and k not in present)
    unjustified = [k for k in missing if k not in ALLOWED_MISSING]

    print(f"sections vides : {len(empty)}")
    for e in empty:
        print("  -", e)
    print(f"clés de Tab.cpp absentes du layout : {len(missing)} ({len(unjustified)} non justifiées)")
    for k in missing:
        print("  -", k, "" if k in unjustified else "(widget dédié)")
    print(f"marqueurs dynamiques restants : {len(dynamic)}")
    for d in dynamic:
        print("  -", d)
    return 1 if empty or unjustified else 0


if __name__ == "__main__":
    sys.exit(main())
