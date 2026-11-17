#!/usr/bin/env python3
"""Listes des assistants « Créer un filament / une imprimante » d'OrcaSlicer
(GUI/CreatePresetsDialog.cpp : filament_vendors, filament_types,
printer_vendors, printer_model_map, nozzle_diameter_vec) extraites des sources
Orca vers audit/create_preset_lists.json (embarqué par le backend,
GET /api/presets/create-options). Les types/fournisseurs des presets système
importés sont ajoutés à l'exécution par le backend, comme le dialogue d'Orca
(m_system_filament_types_set).

Usage : extract_create_preset_lists.py [ORCA_SRC]
"""
import json
import os
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
src = Path(sys.argv[1] if len(sys.argv) > 1 else os.environ.get("ORCA_SRC", ""))
text = (src / "src/slic3r/GUI/CreatePresetsDialog.cpp").read_text(encoding="utf-8")
strings = re.compile(r'"((?:[^"\\]|\\.)*)"')


def block(name: str) -> str:
    m = re.search(rf"{name}\s*=\s*\{{", text)
    if not m:
        raise SystemExit(f"{name} introuvable dans CreatePresetsDialog.cpp")
    depth, i = 0, m.end() - 1
    while True:
        if text[i] == "{":
            depth += 1
        elif text[i] == "}":
            depth -= 1
            if depth == 0:
                return text[m.end():i]
        i += 1


models = {}
for entry in re.finditer(r'\{\s*"([^"]+)"\s*,\s*\{(.*?)\}\s*\}', block("printer_model_map"), re.S):
    models[entry.group(1)] = strings.findall(entry.group(2))

out = {
    "source": "OrcaSlicer src/slic3r/GUI/CreatePresetsDialog.cpp",
    "filament_vendors": strings.findall(block("filament_vendors")),
    "filament_types": strings.findall(block("filament_types")),
    "printer_vendors": strings.findall(block("printer_vendors")),
    "printer_models": models,
    "nozzle_diameters": strings.findall(block("nozzle_diameter_vec")),
}
(HERE / "create_preset_lists.json").write_text(json.dumps(out, indent=1, ensure_ascii=False) + "\n")
print({k: len(v) for k, v in out.items() if k != "source"})
