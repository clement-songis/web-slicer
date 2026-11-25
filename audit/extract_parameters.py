#!/usr/bin/env python3
"""Extrait tous les ConfigOptionDef de src/libslic3r/PrintConfig.cpp.

Sortie : audit/parameters.json
  { "key": { type, label, tooltip, category, mode, default, min, max, ... } }

Ré-exécutable : parse le source C++ à chaque exécution.

Outre les add("clé", coType) littéraux, trois mécanismes dynamiques d'Orca
main (aefb6b3d) enregistrent des options sans littéral de clé — parsés ici,
sinon l'inventaire les perd :
  - macro new_def(key, type, label, tooltip) : placeholders d'état et
    statistiques (valeurs calculées au runtime, sans défaut) ;
  - lambda auto NAME = [this](…) { this->add(param, …) } : add_belt_remap,
    qui enregistre gcode_remap_x/y/z (label/mode/défaut en argument) ;
  - boucle for (const char* key : {"a", …}) : une option par clé littérale
    de la liste (*_plugin_config_overrides).
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

# --- Définitions dynamiques (Orca main aefb6b3d) ----------------------------
# #define new_def(OPT_KEY, TYPE, LABEL, TOOLTIP) : l'expansion est un add()
# dont la clé n'est pas un littéral (invisible pour ADD_RE).
NEWDEF_RE = re.compile(r"\bnew_def\s*\(")
# auto NAME = [this](…) { … this->add(param, …) … } : lambda d'enregistrement.
LAMBDA_DEF_RE = re.compile(r"auto\s+(\w+)\s*=\s*\[\s*this\s*\]\s*\(")
# for (const char* key : {"a", "b", …}) { … this->add(key, …) … }
FOR_KEYLIST_RE = re.compile(
    r"for\s*\(\s*const\s+(?:char\s*\*+|std::string\s*&?)\s*(\w+)\s*:\s*\{"
)
# add(param, coType) dans le corps d'une lambda / boucle.
ADD_PARAM_RE = re.compile(r"this->add(_nullable)?\s*\(\s*\w+\s*,\s*(co\w+)\s*\)")
# Variables texte locales (point_tooltip = L("…")) passées à new_def.
SYMBOL_L_RE = re.compile(
    r'(?<![>\w])([A-Za-z_]\w*)\s*=\s*(L\(\s*"(?:[^"\\]|\\.)*"\s*(?:"(?:[^"\\]|\\.)*"\s*)*\))'
)


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
    # Nullable initialisé à nil (`ConfigOptionFloatsNullable::nil_value()`) :
    # pas de valeur par défaut (l'option hérite).
    if "nil_value()" in args:
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
            # Éléments séparés par les virgules hors guillemets ; des littéraux
            # adjacents (« "a" "b" ») forment UN élément (concaténation C++).
            items, cur, quoted, esc = [], "", False, False
            for ch in m.group(1):
                if quoted:
                    cur += ch
                    if esc:
                        esc = False
                    elif ch == "\\":
                        esc = True
                    elif ch == '"':
                        quoted = False
                elif ch == '"':
                    quoted = True
                    cur += ch
                elif ch == ",":
                    items.append(cur)
                    cur = ""
                else:
                    cur += ch
            if cur.strip():
                items.append(cur)
            return ["".join(re.findall(r'"((?:[^"\\]|\\.)*)"', it)) for it in items if '"' in it]
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
    if ctor == "Point":
        # Vec2d(50, 50) → [50, 50] (sérialisé « 50x50 » par le registre).
        m = re.match(r"^Vec2d\s*\(\s*([-\d.eE+]+)\s*,\s*([-\d.eE+]+)\s*\)$", args)
        if m:
            return [parse_number(m.group(1), consts), parse_number(m.group(2), consts)]
        return args
    if ctor == "Points":
        # { Vec2d(0, 0), Vec2d(200, 0) } → ["0x0", "200x0"] (forme texte d'Orca).
        pts = re.findall(r"Vec2d\s*\(\s*([-\d.eE+]+)\s*,\s*([-\d.eE+]+)\s*\)", args)
        if pts or args.strip("{} ") == "":
            return [f"{parse_number(x, consts)}x{parse_number(y, consts)}" for x, y in pts]
        return args
    # Types numériques scalaires ou vecteurs : Float, Int, Percent, Floats{..}, Ints{..}
    m = re.match(r"^\{(.*)\}$", args, re.S)
    if m:
        items = [x.strip() for x in m.group(1).split(",") if x.strip()]
        parsed = []
        for x in items:
            # Littéraux flottants C++ suffixés (140.f, 0.01f).
            if re.fullmatch(r"-?[\d.]+[fF]", x):
                x = x.rstrip("fF")
                x = x + "0" if x.endswith(".") else x
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


def split_top(expr: str) -> list[str]:
    """Découpe une liste d'arguments C++ aux virgules de profondeur 0
    (sensible aux littéraux et aux parenthèses/accolades imbriquées)."""
    parts: list[str] = []
    buf: list[str] = []
    depth = 0
    i, n = 0, len(expr)
    str_re = re.compile(r'"(?:[^"\\]|\\.)*"')
    while i < n:
        c = expr[i]
        if c == '"':
            m = str_re.match(expr, i)
            buf.append(m.group(0))
            i = m.end()
            continue
        if c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
        if c == "," and depth == 0:
            parts.append("".join(buf).strip())
            buf = []
        else:
            buf.append(c)
        i += 1
    tail = "".join(buf).strip()
    if tail:
        parts.append(tail)
    return parts


def resolve_text(expr: str, sym_map: dict[str, str]) -> str | None:
    """L("a" "b") / "c" / identifiant local (point_tooltip) → texte concaténé."""
    s = concat_strings(expr)
    if s is not None:
        return s
    val = sym_map.get(expr.strip())
    return concat_strings(val) if val is not None else None


def resolve_enum_default(entry: dict, enum_keys: dict) -> None:
    """Résout les défauts d'enum (Enum<T>(Symbole) → clé persistée)."""
    dt = entry.get("default_type", "")
    mt = re.match(r"^Enums?(?:Nullable)?<(\w+)>$", dt)
    if mt and isinstance(entry.get("default"), str):
        table = enum_keys.get(mt.group(1), {})
        raw = entry["default"].strip()
        if raw.startswith("{") and raw.endswith("}"):  # vecteur d'enums
            items = [x.strip().split("::")[-1] for x in raw[1:-1].split(",") if x.strip()]
            if items and all(x in table for x in items):
                entry["default"] = [table[x] for x in items]
        elif raw.split("::")[-1] in table:
            entry["default"] = table[raw.split("::")[-1]]
    # EnumsGeneric{ ZHopType::zhtSlope } / { (int)Overhang_threshold_bridge } :
    # vecteur d'enums sans paramètre de type → table déduite du préfixe, ou
    # recherche de l'énumérateur dans toutes les tables.
    if dt == "EnumsGeneric" and isinstance(entry.get("default"), str):
        raw = entry["default"].strip().strip("{}")
        keys = []
        for item in [x.strip() for x in raw.split(",") if x.strip()]:
            item = re.sub(r"^\(\s*int\s*\)", "", item).replace(" ", "")
            prefix, _, name = item.rpartition("::")
            table = enum_keys.get(prefix) if prefix else None
            if not table:
                table = next((t for t in enum_keys.values() if name in t), {})
            if name in table:
                keys.append(table[name])
        if keys:
            entry["default"] = keys


def parse_key_loop(code: str, m: re.Match) -> tuple[list[str], str] | None:
    """Boucle for (const char* key : {"a", …}) enregistrant des options →
    (clés littérales, corps) si la boucle appelle this->add(var, …)."""
    try:
        close_list = code.index("}", m.end())
        keys = re.findall(r'"([^"]+)"', code[m.end() : close_list])
        open_brace = code.find("{", close_list)
        close_brace = find_matching_paren(code, open_brace, "{", "}")
    except ValueError:
        return None
    body = code[open_brace + 1 : close_brace]
    var = m.group(1)
    if not re.search(rf"this->add(?:_nullable)?\s*\(\s*{re.escape(var)}\s*,", body):
        return None
    return keys, body


def parse_add_lambda(code: str, m: re.Match) -> tuple[str, list[str], str] | None:
    """auto NAME = [this](params) { … this->add(…) … } → (nom, noms des
    paramètres, corps) si la lambda enregistre des options."""
    open_paren = m.end() - 1
    close_paren = find_matching_paren(code, open_paren)
    param_names = []
    for p in split_top(code[open_paren + 1 : close_paren]):
        pm = re.search(r"([A-Za-z_]\w*)\s*$", p)
        if pm:
            param_names.append(pm.group(1))
    open_brace = code.find("{", close_paren)
    if open_brace < 0:
        return None
    close_brace = find_matching_paren(code, open_brace, "{", "}")
    body = code[open_brace + 1 : close_brace]
    if not re.search(r"this->add(?:_nullable)?\s*\(", body):
        return None
    return m.group(1), param_names, body


def extract_block_attrs(block: str, consts: dict, sym_map: dict | None = None) -> dict:
    """Extrait les attributs def->xxx = ...; d'un bloc de définition."""
    attrs: dict = {}

    for name in STR_ATTRS:
        # Jusqu'au premier « ; » hors littéral : une fin de ligne suivie d'un
        # commentaire retiré (espaces) ou un « ; » dans une chaîne ne doivent
        # pas faire déborder sur l'instruction suivante (sidetext pollués par
        # le tooltip / full_label / ratio_over voisins).
        m = re.search(rf'def->{name}\s*=\s*((?:[^;"]|"(?:[^"\\]|\\.)*")*);', block, re.S)
        if m:
            s = concat_strings(m.group(1))
            if s is None and sym_map is not None:
                # Variable texte locale (external_flow_tooltip = L("…")) :
                # résolue via la carte des symboles du fichier.
                s = resolve_text(m.group(1), sym_map)
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

    # Orca utilise indifféremment push_back et emplace_back pour les listes d'enum.
    enum_values = re.findall(
        r'def->enum_values\.(?:push_back|emplace_back)\(\s*(?:L\(\s*)?"((?:[^"\\]|\\.)*)"\s*\)?\s*\)',
        block,
    )
    # Liste littérale : `def->enum_values = {"a", "b", …};`.
    if not enum_values:
        m = re.search(r"def->enum_values\s*=\s*\{(.*?)\}\s*;", block, re.S)
        if m:
            enum_values = re.findall(r'"((?:[^"\\]|\\.)*)"', m.group(1))
    # Liste partagée avec une autre option : `def->enum_values = def_xxx->enum_values;`.
    m = re.search(r"def->enum_values\s*=\s*(\w+)->enum_values\s*;", block)
    if m and not enum_values:
        attrs["enum_values_from"] = m.group(1)
    if enum_values:
        attrs["enum_values"] = enum_values
    enum_labels = [
        concat_strings(e)
        for e in re.findall(r"def->enum_labels\.(?:push_back|emplace_back)\((.*?)\);", block, re.S)
    ]
    enum_labels = [e for e in enum_labels if e is not None]
    if not enum_labels:
        m = re.search(r"def->enum_labels\s*=\s*\{(.*?)\}\s*;", block, re.S)
        if m:
            enum_labels = re.findall(r'"((?:[^"\\]|\\.)*)"', m.group(1))
    if enum_labels:
        attrs["enum_labels"] = enum_labels

    # Défaut : def->set_default_value(new ConfigOptionX<T>(...));
    m = re.search(r"def->set_default_value\s*\(\s*new\s+ConfigOption(\w+)(?:<(\w+)>)?\s*", block)
    if m:
        ctor, enum_t = m.group(1), m.group(2)
        # Arguments : `ConfigOptionX(...)` ou initialisation par accolades
        # `ConfigOptionX{...}` (ex. precise_outer_wall : ConfigOptionBool{true}).
        # Avant, seule la parenthèse était cherchée : pour `{true}` on lisait la
        # parenthèse suivante du bloc et le défaut tombait à null.
        rest = block[m.end():]
        lead = len(rest) - len(rest.lstrip())
        first = rest.lstrip()[:1]
        open_pos = -1
        if first == "{":
            open_pos = m.end() + lead
            depth, close_pos = 0, -1
            for i in range(open_pos, len(block)):
                if block[i] == "{":
                    depth += 1
                elif block[i] == "}":
                    depth -= 1
                    if depth == 0:
                        close_pos = i
                        break
            inner = block[open_pos + 1 : close_pos].strip()
            # Vecteurs (Floats, Ints, Bools…) : la liste d'initialisation reste
            # entre accolades ; scalaires : l'unique valeur.
            vector = ctor.endswith("s") or ctor.endswith("sNullable")
            raw_args = ("{" + inner + "}") if vector and not inner.startswith("{") else inner
            if vector and inner.startswith("{"):
                raw_args = inner
            attrs["default_type"] = ctor + (f"<{enum_t}>" if enum_t else "")
            attrs["default_raw"] = raw_args
            attrs["default"] = parse_default(ctor, raw_args, consts)
        elif first == "(":
            open_pos = m.end() + lead
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


def load_enum_keys_maps(code: str) -> dict[str, dict[str, str]]:
    """`static const t_config_enum_values s_keys_map_<T> = { {"clé", enumérateur}, … }` →
    {T: {enumérateur: clé}} (défauts d'enum exprimés en clés Orca, comme
    `ConfigOptionEnum<T>::serialize`)."""
    maps: dict[str, dict[str, str]] = {}
    # Deux styles dans Orca : `= { … };` et initialisation directe `{ … };`.
    for m in re.finditer(r"s_keys_map_(\w+)\s*=?\s*\{(.*?)\n\s*\};", code, re.S):
        pairs = re.findall(
            r'\{\s*"((?:[^"\\]|\\.)*)"\s*,\s*(?:int\s*\(\s*)?(?:\w+::)*(\w+)\s*\)?\s*\}', m.group(2)
        )
        table: dict[str, str] = {}
        for key, enumerator in pairs:
            table.setdefault(enumerator, key)  # premier alias = clé canonique
        maps[m.group(1)] = table
    return maps


def main() -> None:
    code = strip_comments(read_source(SOURCE))
    enum_keys = load_enum_keys_maps(code)

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

    # Variables texte locales (point_tooltip = L("…")) : label/tooltip des
    # new_def passés par identifiant.
    sym_map: dict[str, str] = dict(SYMBOL_L_RE.findall(code))

    # Lambdas et boucles for qui enregistrent des options : leurs corps sont
    # découpés et re-associés aux appels (new_def/lambda/boucle ci-dessous).
    lambda_data = []  # (match de définition, nom, paramètres, corps)
    for m in LAMBDA_DEF_RE.finditer(code):
        parsed = parse_add_lambda(code, m)
        if parsed is not None:
            lambda_data.append((m, *parsed))
    key_loop_data = []  # (match de boucle, clés littérales, corps)
    for m in FOR_KEYLIST_RE.finditer(code):
        parsed = parse_key_loop(code, m)
        if parsed is not None:
            key_loop_data.append((m, *parsed))

    matches = list(ADD_RE.finditer(code))
    boundaries = [m.start() for m in matches] + [len(code)]
    func_starts = sorted(s for s, _, _ in funcs)
    # Un corps de lambda ou de boucle for sur clés contient des def->… qui
    # n'appartiennent pas à l'option littérale précédente : sans coupure,
    # preset_name héritait du bloc *_plugin_config_overrides et
    # belt_frame_tilt_angle des enum_values de la lambda gcode_remap.
    cut_starts = sorted([m.start() for m, *_ in lambda_data]
                        + [m.start() for m, *_ in key_loop_data])

    params: dict[str, dict] = {}
    for i, m in enumerate(matches):
        block_end = boundaries[i + 1]
        # Ne pas déborder sur la fonction suivante
        for fs in func_starts:
            if m.end() < fs < block_end:
                block_end = fs
                break
        # Ni sur une lambda / boucle d'enregistrement dynamique
        for cp in cut_starts:
            if m.end() < cp < block_end:
                block_end = cp
                break
        block = code[m.end() : block_end]

        entry: dict = {
            "type": m.group("type"),
            "group": group_at(m.start()),
            "line": line_of(code, m.start()),
        }
        if m.group("nullable"):
            entry["nullable"] = True
        entry.update(extract_block_attrs(block, consts, sym_map))
        resolve_enum_default(entry, enum_keys)
        if "mode" not in entry:
            # ConfigOptionDef::mode vaut comSimple par défaut (Config.hpp)
            entry["mode"] = "simple"
            entry["mode_is_default"] = True
        params[m.group("key")] = entry

    # `def_xxx = def = this->add("clé", …)` : variables C++ réutilisées pour partager
    # les listes d'enum (`def->enum_values = def_xxx->enum_values;`).
    var_keys = dict(re.findall(r"(\w+)\s*=\s*def\s*=\s*this->add\(\s*\"(\w+)\"", code))
    for entry in params.values():
        src = entry.pop("enum_values_from", None)
        source = params.get(var_keys.get(src, ""), {}) if src else {}
        if source.get("enum_values"):
            entry["enum_values"] = list(source["enum_values"])
            if source.get("enum_labels") and not entry.get("enum_labels"):
                entry["enum_labels"] = list(source["enum_labels"])

    # --- Définitions dynamiques de main ------------------------------------
    # Traitées après les add() littéraux : en cas de doublon la définition
    # littérale PrintConfigDef gagne (filament_retract_length_nc existe en
    # add() coFloats ET en new_def coFloat ; l'add() est la version preset).

    # Macro new_def(key, type, label, tooltip) : placeholders d'état et
    # statistiques — valeurs calculées au runtime, sans défaut.
    for m in NEWDEF_RE.finditer(code):
        if "#define" in code[max(0, m.start() - 8) : m.start()]:
            continue  # la définition de la macro elle-même
        close = find_matching_paren(code, m.end() - 1)
        args = split_top(code[m.end() : close])
        if len(args) < 4:
            continue
        km = re.match(r'^"([^"]+)"$', args[0].strip())
        tm = re.match(r"^(co\w+)$", args[1].strip())
        if not km or not tm:
            continue
        key, ctype = km.group(1), tm.group(1)
        if key in params:
            continue
        entry: dict = {
            "type": ctype,
            "group": group_at(m.start()),
            "line": line_of(code, m.start()),
        }
        for field, arg in (("label", args[2]), ("tooltip", args[3])):
            text = resolve_text(arg, sym_map)
            if text:
                entry[field] = text
        # La macro ne pose ni mode ni défaut : comSimple s'applique.
        entry["mode"] = "simple"
        entry["mode_is_default"] = True
        params[key] = entry

    # Lambda d'enregistrement (add_belt_remap → gcode_remap_x/y/z) : le corps
    # porte les attributs communs (category, enum_values/labels, type du
    # set_default_value), chaque appel fournit clé/label/tooltip/mode/défaut.
    for m, name, param_names, body in lambda_data:
        attrs = extract_block_attrs(body, consts, sym_map)
        am = ADD_PARAM_RE.search(body)
        if am is None:
            continue

        def param_index(pattern: str) -> int | None:
            """Index du paramètre de la lambda assigné à def->champ."""
            fm = re.search(pattern, body)
            if fm and fm.group(1) in param_names:
                return param_names.index(fm.group(1))
            return None

        label_i = param_index(r"def->label\s*=\s*([A-Za-z_]\w*)\s*;")
        tooltip_i = param_index(r"def->tooltip\s*=\s*([A-Za-z_]\w*)\s*;")
        mode_i = param_index(r"def->mode\s*=\s*([A-Za-z_]\w*)\s*;")
        dm = re.search(
            r"def->set_default_value\s*\(\s*new\s+ConfigOption(\w+)(?:<(\w+)>)?\s*\(\s*([A-Za-z_]\w*)\s*\)\s*\)",
            body,
        )
        default_i = param_names.index(dm.group(3)) if dm and dm.group(3) in param_names else None

        for cm in re.finditer(rf"\b{re.escape(name)}\s*\(", code):
            close = find_matching_paren(code, cm.end() - 1)
            call_args = split_top(code[cm.end() : close])
            if not call_args:
                continue
            km = re.match(r'^"([^"]+)"$', call_args[0].strip())
            if not km or km.group(1) in params:
                continue
            entry = {
                "type": am.group(2),
                "group": group_at(cm.start()),
                "line": line_of(code, cm.start()),
            }
            if am.group(1):
                entry["nullable"] = True
            if label_i is not None and len(call_args) > label_i:
                text = resolve_text(call_args[label_i], sym_map)
                if text:
                    entry["label"] = text
            if tooltip_i is not None and len(call_args) > tooltip_i:
                text = resolve_text(call_args[tooltip_i], sym_map)
                if text:
                    entry["tooltip"] = text
            for k, v in attrs.items():
                if k not in entry:
                    entry[k] = v
            if mode_i is not None and len(call_args) > mode_i:
                mm = re.match(r"^(com\w+)$", call_args[mode_i].strip())
                if mm:
                    entry["mode"] = MODE_MAP.get(mm.group(1), mm.group(1))
            if default_i is not None and len(call_args) > default_i:
                raw = call_args[default_i].strip()
                if dm:
                    entry["default_type"] = dm.group(1) + (
                        f"<{dm.group(2)}>" if dm.group(2) else ""
                    )
                entry["default_raw"] = raw
                entry["default"] = raw  # symbole résolu par resolve_enum_default
            if "mode" not in entry:
                entry["mode"] = "simple"
                entry["mode_is_default"] = True
            resolve_enum_default(entry, enum_keys)
            params[km.group(1)] = entry

    # Boucle for sur une liste de clés littérales (*_plugin_config_overrides) :
    # le corps est le gabarit commun des options enregistrées.
    for m, keys, body in key_loop_data:
        attrs = extract_block_attrs(body, consts, sym_map)
        am = ADD_PARAM_RE.search(body)
        if am is None:
            continue
        for key in keys:
            if key in params:
                continue
            entry = {
                "type": am.group(2),
                "group": group_at(m.start()),
                "line": line_of(code, m.start()),
            }
            if am.group(1):
                entry["nullable"] = True
            entry.update(attrs)
            if "mode" not in entry:
                entry["mode"] = "simple"
                entry["mode_is_default"] = True
            resolve_enum_default(entry, enum_keys)
            params[key] = entry

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
        "note": "mode_is_default=true : aucun def->mode explicite, comSimple s'applique (défaut de Config.hpp). "
                "Inclut les définitions dynamiques d'Orca main : macro new_def (placeholders d'état), "
                "lambda add_belt_remap (gcode_remap_x/y/z), boucle for (*_plugin_config_overrides).",
        "count": len(params),
        "mode_counts": dict(sorted(modes.items())),
        "parameters": dict(sorted(params.items())),
    }
    out = write_json("parameters.json", data)
    print(f"{out}: {len(params)} paramètres ({data['mode_counts']})")


if __name__ == "__main__":
    main()
