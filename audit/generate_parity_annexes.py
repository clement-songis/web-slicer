#!/usr/bin/env python3
"""Génère les annexes de parité de la spec depuis les inventaires audit/*.json.

Chaque entrée des inventaires est matérialisée dans des annexes Markdown
normatives, jointes à la spécification. Ré-exécutable : à relancer après
chaque `audit/run_all.py` pour resynchroniser les annexes.

Usage : python3 audit/generate_parity_annexes.py [--out specs/<feature>/annexes]
"""

from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path

from common import AUDIT_DIR, REPO_ROOT

DEFAULT_OUT = REPO_ROOT / "specs" / "001-orcaslicer-web-parity" / "annexes"


def load(name: str) -> dict:
    return json.loads((AUDIT_DIR / name).read_text(encoding="utf-8"))


def md_escape(s) -> str:
    if s is None:
        return ""
    return str(s).replace("|", "\\|").replace("\n", " ")


def fmt_default(v) -> str:
    if v is None:
        return ""
    if isinstance(v, (dict, list)):
        return md_escape(json.dumps(v, ensure_ascii=False))
    return md_escape(v)


def gen_parameters(out_dir: Path) -> int:
    data = load("parameters.json")
    params = data["parameters"]
    by_group: dict[str, list] = defaultdict(list)
    for key, p in params.items():
        by_group[p["group"]].append((key, p))

    lines = [
        "# Annexe A — Registre exhaustif des paramètres (normatif)",
        "",
        f"Source : `audit/parameters.json` ({data['count']} paramètres, généré depuis",
        "`vendor/OrcaSlicer/src/libslic3r/PrintConfig.cpp`). Chaque ligne est une",
        "exigence : le paramètre DOIT être exposé par l'application (stockage,",
        "API, UI selon son groupe) ou inscrit au registre d'exclusions avec",
        "justification.",
        "",
        "Portée par groupe : `fff`/`common` → exposés dans l'UI de réglages ;",
        "`sla` → registre + API (pas d'UI, OrcaSlicer n'expose pas d'onglet SLA) ;",
        "`cli:*` → équivalents des actions serveur (tranchage, transformations) ;",
        "`other:*` → placeholders G-code et états de slicing (moteur de templates).",
        "",
    ]
    order = sorted(by_group, key=lambda g: ({"common": 0, "fff": 1, "sla": 2}.get(g, 3), g))
    for group in order:
        entries = sorted(by_group[group])
        lines += [f"## Groupe `{group}` ({len(entries)} paramètres)", ""]
        lines += ["| Clé | Type | Catégorie | Mode | Défaut | Libellé |",
                  "|---|---|---|---|---|---|"]
        for key, p in entries:
            lines.append(
                f"| `{key}` | {p.get('type','')} | {md_escape(p.get('category',''))} "
                f"| {p.get('mode','')} | {fmt_default(p.get('default'))} "
                f"| {md_escape(p.get('label',''))} |"
            )
        lines.append("")
    (out_dir / "annexe-a-parametres.md").write_text("\n".join(lines), encoding="utf-8")
    return data["count"]


def gen_ui(out_dir: Path) -> dict:
    ui = load("ui_inventory.json")
    lines = [
        "# Annexe B — Inventaire exhaustif de l'interface (normatif)",
        "",
        "Source : `audit/ui_inventory.json`. Chaque élément DOIT exister dans",
        "l'application web (même organisation, mêmes groupes) ou figurer au",
        "registre d'exclusions avec justification.",
        "",
        "## B.1 Onglets de réglages",
        "",
    ]
    for page in ui["settings_tabs"]:
        title = page["title"] or f"(page dynamique — {page['defined_in']})"
        lines.append(f"### {title} — `{page['defined_in']}`")
        lines.append("")
        for section in page["sections"]:
            opts = ", ".join(
                f"`{o}`" if isinstance(o, str) else f"*dynamique:{o['dynamic']}*"
                for o in section["options"]
            )
            lines.append(f"- **{section['title']}** : {opts if opts else '(widgets spécifiques)'}")
        lines.append("")

    lines += ["## B.2 Menus principaux", ""]
    lines += ["| Menu (variable) | Libellé | Raccourci |", "|---|---|---|"]
    for item in ui["main_menus"]["items"]:
        lines.append(f"| {md_escape(item['menu_variable'])} | {md_escape(item['label'])} "
                     f"| {md_escape(item.get('shortcut') or '')} |")
    lines += ["", "## B.3 Menus contextuels du plateau", ""]
    lines += ["| Menu (variable) | Libellé |", "|---|---|"]
    for item in ui["plater_context_menus"]:
        lines.append(f"| {md_escape(item['menu_variable'])} | {md_escape(item['label'])} |")

    lines += ["", "## B.4 Barres d'outils 3D", ""]
    for func, items in ui["plater_toolbars"].items():
        lines.append(f"- `{func}` : " + ", ".join(f"`{i['name']}`" for i in items))
    lines += ["", "## B.5 Gizmos", ""]
    for g in ui["plater_gizmos"]:
        lines.append(f"- **{g.get('type', g['class'])}** (`{g['class']}`)")

    lines += ["", "## B.6 Raccourcis clavier", ""]
    for group in ui["keyboard_shortcuts"]:
        lines.append(f"### {group['group']}")
        lines.append("")
        lines += ["| Touches | Action |", "|---|---|"]
        for s in group["shortcuts"]:
            lines.append(f"| `{md_escape(s['keys'])}` | {md_escape(s['action'])} |")
        lines.append("")
    (out_dir / "annexe-b-interface.md").write_text("\n".join(lines), encoding="utf-8")
    return ui["summary"]


def gen_presets(out_dir: Path) -> dict:
    inv = load("presets_inventory.json")
    lines = [
        "# Annexe C — Inventaire des profils système (normatif)",
        "",
        "Source : `audit/presets_inventory.json`. La liste nominative complète des",
        f"{inv['summary']['presets_total']} presets est portée par ce fichier JSON,",
        "incorporé par référence : chaque preset qu'il liste DOIT être importé et",
        "utilisable (héritage résolu) — le contrôle se fait par comptage exact",
        "type par type via `audit/run_all.py`, pas par relecture manuelle.",
        "",
        f"## C.1 Totaux normatifs",
        "",
        f"- Presets : {inv['summary']['presets_total']}",
        f"- Par type : {json.dumps(inv['summary']['presets_by_type'])}",
        f"- Instanciables (visibles utilisateur) : {json.dumps(inv['summary']['instantiated_by_type'])}",
        "",
        "## C.2 Vendeurs (chaque ligne est une exigence d'import)",
        "",
        "| Vendeur | Version | machine_model | machine | filament | process |",
        "|---|---|---|---|---|---|",
    ]
    for vendor, v in sorted(inv["vendors"].items()):
        c = v["counts"]
        lines.append(f"| {vendor} | {v.get('version','')} | {c.get('machine_model',0)} "
                     f"| {c.get('machine',0)} | {c.get('filament',0)} | {c.get('process',0)} |")
    lines += [
        "",
        "## C.3 Structure d'un preset (champs méta observés)",
        "",
        "```json",
        json.dumps(inv["structure_notes"]["meta_fields_by_type"], indent=2, ensure_ascii=False),
        "```",
        "",
        "Héritage : champ `inherits` (chaîne parent) ; presets abstraits :",
        "`instantiation=false` ; presets utilisateur : mêmes champs, `from=user`.",
    ]
    (out_dir / "annexe-c-presets.md").write_text("\n".join(lines), encoding="utf-8")
    return inv["summary"]


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)

    n_params = gen_parameters(args.out)
    ui_summary = gen_ui(args.out)
    presets_summary = gen_presets(args.out)
    print(f"{args.out}: annexe A ({n_params} paramètres), "
          f"annexe B ({ui_summary}), annexe C ({presets_summary['presets_total']} presets)")


if __name__ == "__main__":
    main()
