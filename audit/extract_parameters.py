#!/usr/bin/env python3
"""Extrait tous les ConfigOptionDef de src/libslic3r/PrintConfig.cpp.

Sortie : audit/parameters.json
  { "key": { type, label, tooltip, category, mode, default, min, max, ... } }

Ré-exécutable : parse le source C++ à chaque exécution.
"""

from __future__ import annotations

import re

from common import (
    concat_strings,
    find_matching_paren,
    line_of,
    read_source,
    strip_comments,
    write_json,
)

SOURCE = "src/libslic3r/PrintConfig.cpp"
CONSTANTS_HEADER = "src/libslic3r/PrintConfigConstants.hpp"

MODE_MAP = {
    "comSimple": "simple",
    "comAdvanced": "advanced",
    "comExpert": "expert",
    "comDevelop": "develop",
}

INIT_GROUP_MAP = {
    "init_common_params": "common",
    "init_fff_params": "fff",
    "init_sla_params": "sla",
}

ADD_RE = re.compile(
    r'def\s*=\s*this->add(?P<nullable>_nullable)?\(\s*"(?P<key>[^"]+)"\s*,\s*(?P<type>co\w+)\s*\)'
)
FUNC_RE = re.compile(r"^(?:void\s+)?(\w+)::(~?\w+)\s*\(", re.M)

# Constantes numériques déclarées localement dans les fonctions init_*
CONST_RE = re.compile(r"const\s+(?:int|double|float)\s+(\w+)\s*=\s*([-\d.]+)\s*;")

STR_ATTRS = ["label", "full_label", "category", "tooltip", "sidetext"]
NUM_ATTRS = ["min", "max", "max_literal"]


def parse_number(raw: str, consts: dict[str, float]):
    raw = raw.strip()
    if raw in consts:
        return consts[raw]
    if raw.startswith("-") and raw[1:].strip() in consts:
        v = consts[raw[1:].strip()]
        return -v if isinstance(v, (int, float)) else raw
    lit = raw.rstrip("fF") if re.fullmatch(r"-?[\d.]+[fF]", raw) else raw
    try:
        v = float(lit)
        return int(v) if v.is_integer() and "." not in lit and "e" not in lit.lower() else v
    except ValueError:
        return raw  # expression non résolue : garder le texte brut


def parse_default(ctor: str, args: str, consts: dict):
    """Interprète l'argument du constructeur ConfigOptionX(...) quand c'est simple."""
    args = args.strip()
    if args == "":
        return None
    if args in consts:
        return consts[args]
    if ctor.startswith("Enum"):
        return args  # valeur d'enum C++ (ex: atKeyPassword)
    if ctor in ("Bool", "BoolsNullable", "Bools"):
        if args in ("true", "false"):
            return args == "true"
    if ctor == "String":
        s = concat_strings(args)
        return s if s is not None else args
    if ctor == "Strings":
        m = re.search(r"\{(.*)\}", args, re.S)
        if m:
            return [s for s in re.findall(r'"((?:[^"\\]|\\.)*)"', m.group(1))]
        s = concat_strings(args)
        return [s] if s is not None else args
    if ctor == "FloatOrPercent" or ctor == "FloatsOrPercents":
        m = re.match(r"^([-\d.]+)\s*,\s*(true|false)$", args)
        if m:
            return {"value": float(m.group(1)), "percent": m.group(2) == "true"}
        m = re.match(r"^\{\s*([-\d.]+)\s*,\s*(true|false)\s*\}$", args)
        if m:
            return [{"value": float(m.group(1)), "percent": m.group(2) == "true"}]
        return args
    if ctor == "Point" or ctor == "Points":
        return args
    # Types numériques scalaires ou vecteurs : Float, Int, Percent, Floats{..}, Ints{..}
    m = re.match(r"^\{(.*)\}$", args, re.S)
    if m:
        items = [x.strip() for x in m.group(1).split(",") if x.strip()]
        parsed = []
        for x in items:
            try:
                parsed.append(float(x) if "." in x else int(x))
            except ValueError:
                if x in ("true", "false"):
                    parsed.append(x == "true")
                else:
                    return args
        return parsed
    try:
        return float(args) if "." in args else int(args)
    except ValueError:
        return args


def extract_block_attrs(block: str, consts: dict) -> dict:
    """Extrait les attributs def->xxx = ...; d'un bloc de définition."""
    attrs: dict = {}

    for name in STR_ATTRS:
        m = re.search(rf"def->{name}\s*=\s*(.*?);\n", block, re.S)
        if m:
            s = concat_strings(m.group(1))
            if s is not None:
                attrs[name] = s

    for name in NUM_ATTRS:
        m = re.search(rf"def->{name}\s*=\s*([^;]+);", block)
        if m:
            attrs[name] = parse_number(m.group(1), consts)

    m = re.search(r"def->mode\s*=\s*(com\w+)", block)
    if m:
        attrs["mode"] = MODE_MAP.get(m.group(1), m.group(1))

    m = re.search(r"def->gui_type\s*=\s*ConfigOptionDef::GUIType::(\w+)", block)
    if m:
        attrs["gui_type"] = m.group(1)

    if re.search(r"def->readonly\s*=\s*true", block):
        attrs["readonly"] = True
    if re.search(r"def->cli\s*=\s*ConfigOptionDef::nocli", block):
        attrs["cli"] = "nocli"
    m = re.search(r"def->ratio_over\s*=\s*\"([^\"]*)\"", block)
    if m:
        attrs["ratio_over"] = m.group(1)

    enum_values = re.findall(r'def->enum_values\.push_back\(\s*"((?:[^"\\]|\\.)*)"\s*\)', block)
    if enum_values:
        attrs["enum_values"] = enum_values
    enum_labels = [
        concat_strings(e)
        for e in re.findall(r"def->enum_labels\.push_back\((.*?)\);", block, re.S)
    ]
    enum_labels = [e for e in enum_labels if e is not None]
    if enum_labels:
        attrs["enum_labels"] = enum_labels

    # Défaut : def->set_default_value(new ConfigOptionX<T>(...));
    m = re.search(r"def->set_default_value\s*\(\s*new\s+ConfigOption(\w+)(?:<(\w+)>)?\s*", block)
    if m:
        ctor, enum_t = m.group(1), m.group(2)
        open_pos = block.find("(", m.end() - 1)
        # m.end() peut tomber juste avant '(' des arguments
        open_pos = block.find("(", m.end())
        if open_pos != -1:
            close_pos = find_matching_paren(block, open_pos)
            raw_args = block[open_pos + 1 : close_pos].strip()
            attrs["default_type"] = ctor + (f"<{enum_t}>" if enum_t else "")
            attrs["default_raw"] = raw_args
            attrs["default"] = parse_default(ctor, raw_args, consts)

    return attrs


def load_macro_constants() -> dict:
    """#define de PrintConfigConstants.hpp utilisés comme valeurs par défaut."""
    consts: dict = {}
    for m in re.finditer(r"#define\s+(\w+)\s+(\S+)", strip_comments(read_source(CONSTANTS_HEADER))):
        name, raw = m.group(1), m.group(2)
        if raw in ("true", "false"):
            consts[name] = raw == "true"
        else:
            try:
                v = float(raw)
                consts[name] = int(v) if v.is_integer() and "." not in raw else v
            except ValueError:
                consts[name] = raw
    return consts


def main() -> None:
    code = strip_comments(read_source(SOURCE))

    consts: dict = load_macro_constants()
    for m in CONST_RE.finditer(code):
        try:
            v = float(m.group(2))
            consts[m.group(1)] = int(v) if v.is_integer() else v
        except ValueError:
            pass

    # Bornes des fonctions membres pour taguer le groupe (common/fff/sla/cli/placeholders…)
    funcs = [(m.start(), m.group(1), m.group(2)) for m in FUNC_RE.finditer(code)]

    def group_at(pos: int) -> str:
        current = "unknown"
        for start, cls, func in funcs:
            if start > pos:
                break
            if cls == "PrintConfigDef":
                current = INIT_GROUP_MAP.get(func, func)
            elif cls.startswith("CLI"):
                current = f"cli:{cls.removesuffix('ConfigDef')}"
            else:
                # défs de placeholders G-code, états de slicing, etc.
                current = f"other:{cls.removesuffix('ConfigDef')}"
        return current

    matches = list(ADD_RE.finditer(code))
    boundaries = [m.start() for m in matches] + [len(code)]
    func_starts = sorted(s for s, _, _ in funcs)

    params: dict[str, dict] = {}
    for i, m in enumerate(matches):
        block_end = boundaries[i + 1]
        # Ne pas déborder sur la fonction suivante
        for fs in func_starts:
            if m.end() < fs < block_end:
                block_end = fs
                break
        block = code[m.end() : block_end]

        entry: dict = {
            "type": m.group("type"),
            "group": group_at(m.start()),
            "line": line_of(code, m.start()),
        }
        if m.group("nullable"):
            entry["nullable"] = True
        entry.update(extract_block_attrs(block, consts))
        if "mode" not in entry:
            # ConfigOptionDef::mode vaut comSimple par défaut (Config.hpp)
            entry["mode"] = "simple"
            entry["mode_is_default"] = True
        params[m.group("key")] = entry

    # Définitions dynamiques : surcharges filament_* des options extrudeur
    # (boucle sur filament_extruder_override_keys dans init_filament_option_keys)
    m = re.search(
        r"const\s+std::vector<std::string>\s+filament_extruder_override_keys\s*=\s*\{(.*?)\};",
        code,
        re.S,
    )
    if m:
        override_keys = re.findall(r'"([^"]+)"', m.group(1))
        for key in override_keys:
            if key in params:
                continue
            base_key = key[len("filament_"):] if key.startswith("filament_") else key
            base = params.get(base_key)
            if base is None:
                params[key] = {"group": "fff", "nullable": True, "derived_from": base_key,
                               "note": "clé de base introuvable"}
                continue
            entry = {k: v for k, v in base.items() if k not in ("line", "default", "default_raw", "default_type")}
            entry["type"] = base["type"] + ("Nullable" if not base["type"].endswith("Nullable") else "")
            entry["nullable"] = True
            entry["derived_from"] = base_key
            entry["default"] = None
            entry["note"] = "généré dynamiquement (surcharge filament des réglages extrudeur, défaut nil)"
            params[key] = entry

    # Définitions en boucle sur les axes (PrintConfig.cpp ~4498) :
    # machine_max_{speed,acceleration,jerk}_{x,y,z,e} — clés non littérales,
    # reconstruites depuis le tableau AxisDefault (défauts par axe).
    m = re.search(r"std::vector<AxisDefault>\s+axes\s*\{(.*?)\};", code, re.S)
    if m:
        axis_rows = re.findall(
            r'\{\s*"(\w)"\s*,\s*\{([^}]*)\}\s*,\s*\{([^}]*)\}\s*,\s*\{([^}]*)\}\s*\}',
            m.group(1),
        )
        kinds = [
            ("machine_max_speed_", "Maximum speed %s", "Maximum speed of %s axis", "mm/s", 1),
            ("machine_max_acceleration_", "Maximum acceleration %s",
             "Maximum acceleration of the %s axis", "mm/s²", 2),
            ("machine_max_jerk_", "Maximum jerk %s", "Maximum jerk of the %s axis", "mm/s", 3),
        ]
        for row in axis_rows:
            axis = row[0]
            for prefix, label_fmt, tooltip_fmt, sidetext, idx in kinds:
                nums = []
                for x in row[idx].split(","):
                    v = float(x)
                    nums.append(int(v) if v.is_integer() else v)
                params[prefix + axis] = {
                    "type": "coFloats",
                    "group": "fff",
                    "full_label": label_fmt % axis.upper(),
                    "category": "Machine limits",
                    "tooltip": tooltip_fmt % axis.upper(),
                    "sidetext": sidetext,
                    "min": 0,
                    "mode": "simple",
                    "default_type": "Floats",
                    "default": nums,
                    "note": "généré par la boucle AxisDefault de PrintConfig.cpp (clé non littérale)",
                }

    modes = {}
    for p in params.values():
        modes[p.get("mode", "unset")] = modes.get(p.get("mode", "unset"), 0) + 1

    data = {
        "source": SOURCE,
        "generated_by": "audit/extract_parameters.py",
        "note": "mode_is_default=true : aucun def->mode explicite, comSimple s'applique (défaut de Config.hpp)",
        "count": len(params),
        "mode_counts": dict(sorted(modes.items())),
        "parameters": dict(sorted(params.items())),
    }
    out = write_json("parameters.json", data)
    print(f"{out}: {len(params)} paramètres ({data['mode_counts']})")


if __name__ == "__main__":
    main()
