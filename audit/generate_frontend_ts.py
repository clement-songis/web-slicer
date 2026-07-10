#!/usr/bin/env python3
"""Génère les artefacts TypeScript du frontend depuis l'audit (R2, T040).

Deux fichiers committés (diffs de parité revus en PR), régénérés par
`scripts/codegen.sh` :

  * frontend/src/generated/params.ts    ← audit/parameters.json (858 params)
  * frontend/src/generated/ui-layout.ts ← audit/ui_inventory.json
                                           (21 pages / 100 sections / 525 lignes)

Leur fraîcheur est verrouillée côté frontend par un test `bun:test` qui
reconstruit la structure attendue depuis l'audit et la compare aux constantes
générées.

Sortie déterministe : deux exécutions successives produisent des octets
identiques (clés triées, JSON compact stable).
"""

from __future__ import annotations

import json

from common import AUDIT_DIR, REPO_ROOT

GEN_DIR = REPO_ROOT / "frontend" / "src" / "generated"
PARAMS_OUT = GEN_DIR / "params.ts"
LAYOUT_OUT = GEN_DIR / "ui-layout.ts"

HEADER = (
    "// GÉNÉRÉ par audit/generate_frontend_ts.py (scripts/codegen.sh)\n"
    "// — NE PAS ÉDITER À LA MAIN (constitution V).\n"
    "// Source : {src}.\n"
    "// Fraîcheur verrouillée par {test}.\n"
)

# Les 20 types de config Orca présents dans l'audit (union littérale TS).
PARAM_TYPES = [
    "coBool", "coBools", "coBoolsNullable",
    "coEnum", "coEnums", "coEnumsNullable",
    "coFloat", "coFloats", "coFloatsNullable",
    "coFloatOrPercent", "coFloatsOrPercents",
    "coInt", "coInts",
    "coPercent", "coPercents", "coPercentsNullable",
    "coPoint", "coPoints", "coPointsGroups",
    "coString", "coStrings",
]

MODES = ["simple", "advanced", "expert", "develop"]


def ts(value) -> str:
    """Encode une valeur JSON en littéral TypeScript compact et déterministe."""
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"))


def num_or_null(v):
    """Nombre fini ou None (les bornes absentes → null)."""
    if isinstance(v, bool) or not isinstance(v, (int, float)):
        return None
    return v


def build_params() -> str:
    data = json.load(open(AUDIT_DIR / "parameters.json", encoding="utf-8"))
    params = data["parameters"]

    entries = []
    for key in sorted(params):
        p = params[key]
        rec = {
            "key": key,
            "type": p["type"],
            "mode": p.get("mode", "advanced"),
            "label": p.get("label") or p.get("full_label") or key,
            "tooltip": p.get("tooltip") or "",
            "sidetext": p.get("sidetext") or "",
            "category": p.get("category") or "",
            "group": p.get("group") or "",
            "nullable": bool(p.get("nullable", False)),
            "min": num_or_null(p.get("min")),
            "max": num_or_null(p.get("max")),
            "enumValues": p.get("enum_values") or [],
            "enumLabels": p.get("enum_labels") or [],
            "ratioOver": p.get("ratio_over") or "",
            "default": p.get("default", None),
        }
        entries.append(f"  {ts(key)}: {ts(rec)},")

    body = "\n".join(entries)
    types_union = " | ".join(ts(t) for t in PARAM_TYPES)
    modes_union = " | ".join(ts(m) for m in MODES)
    header = HEADER.format(
        src="audit/parameters.json (858 paramètres)",
        test="frontend/src/lib/settings/params.test.ts",
    )
    return f"""{header}
/** Type de config Orca (miroir de `ConfigOptionType`). */
export type ParamType = {types_union};

/** Mode d'affichage minimal requis pour voir le paramètre. */
export type ParamMode = {modes_union};

/** Définition d'un paramètre du registre (résolution type → widget en T041). */
export interface ParamDef {{
  key: string;
  type: ParamType;
  mode: ParamMode;
  label: string;
  tooltip: string;
  /** Unité affichée à droite du champ (ex. « mm », « % »). */
  sidetext: string;
  category: string;
  group: string;
  nullable: boolean;
  min: number | null;
  max: number | null;
  enumValues: string[];
  enumLabels: string[];
  /** Paramètre de référence d'un pourcentage (FloatOrPercent). */
  ratioOver: string;
  default: unknown;
}}

/** Registre complet, indexé par clé de paramètre. */
export const PARAMS: Record<string, ParamDef> = {{
{body}
}};

/** Nombre de paramètres (contrôle de fraîcheur). */
export const PARAM_COUNT = {len(params)};
"""


def build_layout() -> str:
    data = json.load(open(AUDIT_DIR / "ui_inventory.json", encoding="utf-8"))
    tabs = data["settings_tabs"]
    summary = data["summary"]

    pages = []
    for tab in tabs:
        sections = [
            {"title": s.get("title") or "", "options": list(s.get("options") or [])}
            for s in tab.get("sections") or []
        ]
        pages.append(
            {
                "title": tab.get("title") or "",
                "icon": tab.get("icon") or "",
                "sections": sections,
            }
        )

    lines = ["export const UI_LAYOUT: UiPage[] = ["]
    for page in pages:
        lines.append("  {")
        lines.append(f"    title: {ts(page['title'])},")
        lines.append(f"    icon: {ts(page['icon'])},")
        lines.append("    sections: [")
        for s in page["sections"]:
            lines.append(
                f"      {{ title: {ts(s['title'])}, options: {ts(s['options'])} }},"
            )
        lines.append("    ],")
        lines.append("  },")
    lines.append("];")
    body = "\n".join(lines)

    header = HEADER.format(
        src="audit/ui_inventory.json",
        test="frontend/src/lib/settings/ui-layout.test.ts",
    )
    return f"""{header}
/** Ligne d'option : une clé de `PARAMS` (params.ts), ou un marqueur de ligne
 *  générée dynamiquement par Orca (ex. options par extrudeur/plugin). */
export type UiOption = string | {{ dynamic: string }};

/** Un groupe d'options au sein d'une page. */
export interface UiSection {{
  title: string;
  options: UiOption[];
}}

/** Une page d'onglet de réglages (Quality, Strength, Speed…). */
export interface UiPage {{
  title: string;
  icon: string;
  sections: UiSection[];
}}

{body}

/** Compteurs de structure (contrôle de fraîcheur). */
export const UI_LAYOUT_COUNTS = {{
  pages: {summary['settings_pages']},
  sections: {summary['settings_sections']},
  optionLines: {summary['settings_option_lines']},
}};
"""


def main() -> None:
    GEN_DIR.mkdir(parents=True, exist_ok=True)
    PARAMS_OUT.write_text(build_params(), encoding="utf-8")
    LAYOUT_OUT.write_text(build_layout(), encoding="utf-8")
    print(f"généré {PARAMS_OUT.relative_to(REPO_ROOT)}")
    print(f"généré {LAYOUT_OUT.relative_to(REPO_ROOT)}")


if __name__ == "__main__":
    main()
