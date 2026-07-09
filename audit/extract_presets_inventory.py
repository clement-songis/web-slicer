#!/usr/bin/env python3
"""Inventorie les profils système d'OrcaSlicer (resources/profiles).

Structure : chaque vendeur a un index <Vendor>.json qui référence des presets
de 4 types : machine_model (modèle d'imprimante), machine (imprimante+variante
de buse), filament, process. Les presets héritent les uns des autres via
« inherits » ; « instantiation »: "true" signifie visible comme preset concret.

Sortie : audit/presets_inventory.json
  - vendors  : index par vendeur (version, compteurs)
  - presets  : liste complète (vendeur, type, nom, héritage, compatibilités)
  - key_usage: fréquence de chaque clé de config par type de preset
  - schema   : champs structurels observés par type
"""

from __future__ import annotations

import json
import re
from collections import Counter, defaultdict

from common import ORCA_ROOT, write_json

PROFILES_DIR = ORCA_ROOT / "resources" / "profiles"

# Champs structurels (méta) — tout le reste est considéré comme clé de config.
META_FIELDS = {
    "type", "name", "inherits", "from", "setting_id", "instantiation",
    "version", "url", "description", "filament_id", "printer_model",
    "printer_variant", "compatible_printers", "compatible_printers_condition",
    "sub_path", "family", "model_id", "machine_tech", "bed_model",
    "bed_texture", "default_materials", "default_bed_type", "hotend_model",
    "nozzle_diameter",
}

LIST_KEYS = {
    "machine_model_list": "machine_model",
    "machine_list": "machine",
    "filament_list": "filament",
    "process_list": "process",
}


def load_json(path):
    try:
        return json.loads(path.read_text(encoding="utf-8-sig", errors="replace"))
    except json.JSONDecodeError as e:
        return {"__parse_error__": str(e)}


def summarize_preset(vendor: str, ptype: str, sub_path: str, data: dict) -> dict:
    entry = {
        "vendor": vendor,
        "type": data.get("type", ptype),
        "name": data.get("name"),
        "sub_path": sub_path,
    }
    for field in ("inherits", "from", "setting_id", "filament_id", "instantiation"):
        if field in data:
            entry[field] = data[field]
    if entry.get("instantiation") is not None:
        entry["instantiation"] = str(entry["instantiation"]).lower() == "true"
    if ptype == "machine":
        for field in ("printer_model", "printer_variant", "nozzle_diameter"):
            if field in data:
                entry[field] = data[field]
    if ptype == "machine_model":
        for field in ("family", "machine_tech", "model_id", "nozzle_diameter",
                      "bed_model", "bed_texture", "default_materials"):
            if field in data:
                entry[field] = data[field]
    if "compatible_printers" in data and isinstance(data["compatible_printers"], list):
        entry["compatible_printers"] = data["compatible_printers"]
    n_keys = sum(1 for k in data if k not in META_FIELDS)
    entry["config_key_count"] = n_keys
    if "__parse_error__" in data:
        entry["parse_error"] = data["__parse_error__"]
    return entry


def main() -> None:
    vendors = {}
    presets = []
    key_usage: dict[str, Counter] = defaultdict(Counter)
    schema_fields: dict[str, Counter] = defaultdict(Counter)
    referenced = set()
    errors = []

    index_files = sorted(p for p in PROFILES_DIR.glob("*.json"))
    for index_path in index_files:
        vendor = index_path.stem
        index = load_json(index_path)
        if "__parse_error__" in index:
            errors.append({"file": str(index_path.relative_to(ORCA_ROOT)), "error": index["__parse_error__"]})
            continue
        if vendor == "blacklist":
            continue

        vendor_entry = {
            "display_name": index.get("name"),
            "version": index.get("version"),
            "description": index.get("description"),
            "counts": {},
        }
        for list_key, ptype in LIST_KEYS.items():
            items = index.get(list_key, [])
            vendor_entry["counts"][ptype] = len(items)
            for item in items:
                sub_path = item.get("sub_path", "")
                preset_path = PROFILES_DIR / vendor / sub_path
                referenced.add(preset_path)
                if not preset_path.exists():
                    errors.append({
                        "vendor": vendor,
                        "sub_path": sub_path,
                        "error": "référencé dans l'index mais fichier absent",
                    })
                    continue
                data = load_json(preset_path)
                if "__parse_error__" in data:
                    errors.append({"vendor": vendor, "sub_path": sub_path, "error": data["__parse_error__"]})
                    continue
                presets.append(summarize_preset(vendor, ptype, sub_path, data))
                for k in data:
                    if k in META_FIELDS:
                        schema_fields[ptype][k] += 1
                    else:
                        key_usage[ptype][k] += 1
        vendors[vendor] = vendor_entry

    # fichiers JSON présents sur disque mais référencés nulle part
    orphans = []
    for path in sorted(PROFILES_DIR.rglob("*.json")):
        if path in referenced or path.parent == PROFILES_DIR:
            continue
        if path.name == "cli_config.json":
            continue
        orphans.append(str(path.relative_to(PROFILES_DIR)))

    by_type = Counter(p["type"] for p in presets)
    instantiated = Counter(p["type"] for p in presets if p.get("instantiation"))

    data = {
        "generated_by": "audit/extract_presets_inventory.py",
        "source": "resources/profiles",
        "summary": {
            "vendors": len(vendors),
            "presets_total": len(presets),
            "presets_by_type": dict(sorted(by_type.items())),
            "instantiated_by_type": dict(sorted(instantiated.items())),
            "orphan_files": len(orphans),
            "errors": len(errors),
        },
        "structure_notes": {
            "hierarchy": "vendeur → {machine_model, machine, filament, process} ; "
                         "héritage par champ « inherits », presets abstraits si instantiation=false",
            "meta_fields_by_type": {t: dict(c.most_common()) for t, c in sorted(schema_fields.items())},
        },
        "vendors": vendors,
        "key_usage_by_type": {t: dict(c.most_common()) for t, c in sorted(key_usage.items())},
        "presets": presets,
        "orphan_files": orphans,
        "errors": errors,
    }
    out = write_json("presets_inventory.json", data)
    print(f"{out}: {data['summary']}")


if __name__ == "__main__":
    main()
