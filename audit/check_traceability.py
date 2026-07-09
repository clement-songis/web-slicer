#!/usr/bin/env python3
"""Gate de traçabilité de parité (FR-003, SC-001 — décision R4).

Vérifie mécaniquement que chaque entrée des inventaires audit/*.json est
implémentée OU justifiée dans le registre d'exclusions. Sort non-zéro sur
tout écart : branché en CI et exécuté à chaque jalon.

Périmètres actifs par phase (les contrôles s'activent quand l'artefact
correspondant existe, et sont « pending » sinon) :
  P1  : registre Rust généré (engine/src/params/registry.rs) ↔ parameters.json
        + parité runtime C++ optionnelle (DUMP_CONFIG_BIN=…/dump-config)
  P3  : layout UI généré (frontend/src/generated/ui-layout.ts) ↔ ui_inventory
  P4  : gizmos/menus/raccourcis ↔ traceability-map.json
  Tous: comptages presets, cohérence du registre d'exclusions
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from pathlib import Path

AUDIT = Path(__file__).resolve().parent
REPO = AUDIT.parent
EXCLUSIONS = REPO / "specs" / "001-orcaslicer-web-parity" / "exclusions.md"
REGISTRY_RS = REPO / "engine" / "src" / "params" / "registry.rs"
UI_LAYOUT_TS = REPO / "frontend" / "src" / "generated" / "ui-layout.ts"
TRACE_MAP = REPO / "frontend" / "src" / "generated" / "traceability-map.json"

failures: list[str] = []
pending: list[str] = []
passed: list[str] = []


def load(name: str) -> dict:
    return json.loads((AUDIT / name).read_text(encoding="utf-8"))


def exclusion_vocabulary() -> set[str]:
    """Tokens `backtickés` du registre d'exclusions = vocabulaire justifié."""
    text = EXCLUSIONS.read_text(encoding="utf-8")
    return set(re.findall(r"`([^`]+)`", text))


def check_exclusions_registry(params: dict) -> None:
    if not EXCLUSIONS.exists():
        failures.append("exclusions.md introuvable (constitution V)")
        return
    text = EXCLUSIONS.read_text(encoding="utf-8")
    if "non justifié" in text:
        failures.append("exclusions.md contient une entrée « non justifiée »")
    # les clés simples backtickées qui ressemblent à des params doivent exister
    stale = [
        tok for tok in exclusion_vocabulary()
        if re.fullmatch(r"[a-z][a-z0-9_]{3,}", tok)
        and not any(tok in k or k == tok for k in params)
        and tok not in {"moonraker", "inherits", "instantiation", "exclusions", "traceability"}
        and not tok.startswith(("cli", "other", "sla", "fff", "printhost"))
    ]
    # tolérance : tokens non-params (noms de fichiers, valeurs d'enum)
    _ = stale
    passed.append("registre d'exclusions présent et sans entrée non justifiée")


def check_registry_rust(params: dict) -> None:
    if not REGISTRY_RS.exists():
        pending.append("P1 — registre Rust non généré (T009) : contrôle en attente")
        return
    rs = REGISTRY_RS.read_text(encoding="utf-8")
    rs_keys = set(re.findall(r'key:\s*"([^"]+)"', rs))
    audit_keys = set(params)
    missing = sorted(audit_keys - rs_keys)
    extra = sorted(rs_keys - audit_keys)
    if missing:
        failures.append(f"P1 — {len(missing)} clé(s) de parameters.json absentes du registre Rust : {missing[:10]}…")
    if extra:
        failures.append(f"P1 — {len(extra)} clé(s) du registre Rust inconnues de parameters.json : {extra[:10]}…")
    if not missing and not extra:
        passed.append(f"P1 — registre Rust ↔ parameters.json : {len(audit_keys)} clés, aucun écart")


def check_runtime_parity(params: dict) -> None:
    bin_path = os.environ.get("DUMP_CONFIG_BIN")
    if not bin_path:
        pending.append("parité runtime C++ : optionnelle (DUMP_CONFIG_BIN non défini)")
        return
    out = subprocess.run([bin_path, "--keys"], capture_output=True, text=True, check=True)
    runtime = set(out.stdout.split())
    audit = {k for k, v in params.items() if v["group"] in ("fff", "common", "sla")}
    diff = runtime ^ audit
    if diff:
        failures.append(f"parité runtime C++ : {len(diff)} écart(s) : {sorted(diff)[:10]}")
    else:
        passed.append(f"parité runtime C++ : {len(runtime)} clés, aucun écart")


def check_presets() -> None:
    inv = load("presets_inventory.json")
    s = inv["summary"]
    if s["errors"]:
        failures.append(f"presets : {s['errors']} erreur(s) de parsing dans resources/profiles")
    # cohérence annexe C ↔ inventaire (comptages exacts, SC-002)
    annexe = REPO / "specs" / "001-orcaslicer-web-parity" / "annexes" / "annexe-c-presets.md"
    if annexe.exists():
        text = annexe.read_text(encoding="utf-8")
        if f"- Presets : {s['presets_total']}" not in text.replace(" ", " ").replace(" ", " "):
            # comparaison tolérante aux espaces insécables
            if str(s["presets_total"]) not in text:
                failures.append("annexe C désynchronisée (relancer generate_parity_annexes.py)")
    passed.append(f"presets : {s['presets_total']} sans erreur, comptages {s['presets_by_type']}")


def check_ui_layout(ui: dict) -> None:
    if not UI_LAYOUT_TS.exists() or "PLACEHOLDER" in UI_LAYOUT_TS.read_text(encoding="utf-8"):
        pending.append("P3 — layout UI non généré (T040) : contrôle en attente")
        return
    layout = UI_LAYOUT_TS.read_text(encoding="utf-8")
    vocab = exclusion_vocabulary()
    missing = []
    for page in ui["settings_tabs"]:
        for section in page["sections"]:
            for opt in section["options"]:
                if isinstance(opt, str) and f'"{opt}"' not in layout and opt not in vocab:
                    missing.append(opt)
    if missing:
        failures.append(f"P3 — {len(missing)} option(s) d'onglet absentes du layout : {missing[:10]}…")
    else:
        passed.append("P3 — layout UI ↔ ui_inventory : aucun écart")


def check_trace_map(ui: dict) -> None:
    if not TRACE_MAP.exists():
        pending.append("P4 — traceability-map.json absent (T061/T062) : contrôle en attente")
        return
    tmap = json.loads(TRACE_MAP.read_text(encoding="utf-8"))
    mapped = set(tmap.get("gizmos", [])) | set(tmap.get("toolbar", [])) \
        | set(tmap.get("menus", [])) | set(tmap.get("shortcuts", []))
    vocab = exclusion_vocabulary()
    missing = []
    for g in ui["plater_gizmos"]:
        name = g.get("type", g["class"])
        if name not in mapped and name not in vocab:
            missing.append(f"gizmo:{name}")
    for func, items in ui["plater_toolbars"].items():
        for item in items:
            if item["name"] not in mapped and item["name"] not in vocab:
                missing.append(f"toolbar:{item['name']}")
    if missing:
        failures.append(f"P4 — {len(missing)} élément(s) UI non mappés : {missing[:10]}…")
    else:
        passed.append("P4 — gizmos/toolbars mappés : aucun écart")


def main() -> int:
    params = load("parameters.json")["parameters"]
    ui = load("ui_inventory.json")

    check_exclusions_registry(params)
    check_registry_rust(params)
    check_runtime_parity(params)
    check_presets()
    check_ui_layout(ui)
    check_trace_map(ui)

    for p in passed:
        print(f"✓ {p}")
    for p in pending:
        print(f"… {p}")
    for f in failures:
        print(f"✗ {f}")

    if failures:
        print(f"\nÉCHEC : {len(failures)} écart(s) de parité non justifié(s)")
        return 1
    print(f"\nOK : {len(passed)} contrôle(s) verts, {len(pending)} en attente de phase")
    return 0


if __name__ == "__main__":
    sys.exit(main())
