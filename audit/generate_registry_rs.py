#!/usr/bin/env python3
"""Génère engine/src/params/registry.rs depuis audit/parameters.json (R2, T009).

Le fichier généré est committé (diffs de parité revus en PR) ; sa fraîcheur
est verrouillée par un test du crate engine qui compare le hash SHA-1 de
parameters.json au constant SOURCE_SHA1 embarqué ici.
"""

from __future__ import annotations

import hashlib
import json

from common import AUDIT_DIR, REPO_ROOT

OUT = REPO_ROOT / "engine" / "src" / "params" / "registry.rs"

# type audit (sans suffixe Nullable) → variante ParamKind
KIND_MAP = {
    "coFloat": "Float", "coFloats": "Floats",
    "coInt": "Int", "coInts": "Ints",
    "coBool": "Bool", "coBools": "Bools",
    "coString": "String", "coStrings": "Strings",
    "coPercent": "Percent", "coPercents": "Percents",
    "coFloatOrPercent": "FloatOrPercent", "coFloatsOrPercents": "FloatsOrPercents",
    "coEnum": "Enum", "coEnums": "Enums",
    "coPoint": "Point", "coPoints": "Points", "coPointsGroups": "PointsGroups",
}

GROUP_MAP = {"common": "Common", "fff": "Fff", "sla": "Sla"}
MODE_MAP = {"simple": "Simple", "advanced": "Advanced",
            "expert": "Expert", "develop": "Develop"}


def rs_str(s: str) -> str:
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n") + '"'


def opt_str(v) -> str:
    return f"Some({rs_str(v)})" if isinstance(v, str) and v != "" else "None"


def opt_f64(v) -> str:
    if isinstance(v, bool) or not isinstance(v, (int, float)):
        return "None"
    return f"Some({float(v)}_f64)"


def str_slice(values) -> str:
    if not values:
        return "&[]"
    return "&[" + ", ".join(rs_str(v) for v in values) + "]"


def enum_default(p: dict) -> object:
    """Résout un défaut d'enum C++ (ex. ipCrossHatch) vers sa valeur config."""
    d = p.get("default")
    values = p.get("enum_values") or []
    if not isinstance(d, str) or not values:
        return d
    if d in values:
        return d
    norm = d.lstrip("-")
    # retire le préfixe minuscule hongrois (ip, sm, tst…) puis normalise
    core = norm.lstrip("abcdefghijklmnopqrstuvwxyz") or norm
    flat = core.replace("_", "").replace("-", "").lower()
    matches = [v for v in values if v.replace("_", "").replace("-", "").lower() == flat]
    return matches[0] if len(matches) == 1 else None


def default_json(p: dict) -> str:
    kind = p["type"].removesuffix("Nullable")
    d = enum_default(p) if kind in ("coEnum", "coEnums") else p.get("default")
    if d is None:
        return "None"
    if isinstance(d, str) and not kind.startswith(("coString", "coEnum", "coPoint")):
        return "None"  # expression C++ non résolue
    return f"Some({rs_str(json.dumps(d, ensure_ascii=False))})"


def main() -> None:
    src = (AUDIT_DIR / "parameters.json").read_bytes()
    data = json.loads(src)
    params = data["parameters"]

    rows = []
    for key in sorted(params):
        p = params[key]
        t = p["type"]
        nullable = bool(p.get("nullable")) or t.endswith("Nullable")
        kind = KIND_MAP[t.removesuffix("Nullable")]
        g = p["group"]
        group = GROUP_MAP.get(g, "Cli" if g.startswith("cli:") else "Other")
        rows.append(
            "    ParamDef {\n"
            f"        key: {rs_str(key)},\n"
            f"        kind: ParamKind::{kind},\n"
            f"        nullable: {'true' if nullable else 'false'},\n"
            f"        group: ParamGroup::{group},\n"
            f"        mode: Mode::{MODE_MAP[p.get('mode', 'simple')]},\n"
            f"        category: {opt_str(p.get('category'))},\n"
            f"        label: {opt_str(p.get('label') or p.get('full_label'))},\n"
            f"        tooltip: {opt_str(p.get('tooltip'))},\n"
            f"        sidetext: {opt_str(p.get('sidetext'))},\n"
            f"        min: {opt_f64(p.get('min'))},\n"
            f"        max: {opt_f64(p.get('max'))},\n"
            f"        enum_values: {str_slice(p.get('enum_values'))},\n"
            f"        enum_labels: {str_slice(p.get('enum_labels'))},\n"
            f"        default_json: {default_json(p)},\n"
            f"        readonly: {'true' if p.get('readonly') else 'false'},\n"
            "    },"
        )

    sha1 = hashlib.sha1(src).hexdigest()
    body = f"""// GÉNÉRÉ par audit/generate_registry_rs.py — NE PAS ÉDITER (constitution V).
// Source : audit/parameters.json ({len(params)} paramètres).
// Fraîcheur verrouillée par le test `registre_synchronise_avec_l_audit`.

use super::{{Mode, ParamDef, ParamGroup, ParamKind}};

/// SHA-1 de audit/parameters.json au moment de la génération.
pub const SOURCE_SHA1: &str = "{sha1}";

/// Registre complet, trié par clé (recherche dichotomique via `get`).
pub static REGISTRY: &[ParamDef] = &[
{chr(10).join(rows)}
];
"""
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(body, encoding="utf-8")
    print(f"{OUT}: {len(params)} paramètres, sha1 {sha1[:12]}")


if __name__ == "__main__":
    main()
