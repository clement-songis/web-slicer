#!/usr/bin/env python3
"""Inventorie l'interface d'OrcaSlicer : onglets de réglages, menus,
barres d'outils du Plater et raccourcis clavier.

Sources :
  - src/slic3r/GUI/Tab.cpp            → onglets/sections/options (add_options_page,
                                        new_optgroup, append_single_option_line)
  - src/slic3r/GUI/MainFrame.cpp      → menus principaux (append_menu_item)
  - src/slic3r/GUI/GUI_Factories.cpp  → menus contextuels du Plater
  - src/slic3r/GUI/GLCanvas3D.cpp     → barres d'outils 3D (GLToolbarItem)
  - src/slic3r/GUI/KBShortcutsDialog.cpp → raccourcis clavier documentés

Sortie : audit/ui_inventory.json
"""

from __future__ import annotations

import re

from common import ORCA_ROOT, concat_strings, line_of, read_source, strip_comments, write_json

MEMBER_FUNC_RE = re.compile(r"^[\w:<>*&,\s]*?\b(\w+)::(~?\w+)\s*\([^;{]*\)\s*(?:const\s*)?\{", re.M)


def function_spans(code: str) -> list[tuple[int, str]]:
    """[(position, "Classe::methode"), ...] triés — contexte approximatif par position."""
    return [(m.start(), f"{m.group(1)}::{m.group(2)}") for m in MEMBER_FUNC_RE.finditer(code)]


def context_at(spans: list[tuple[int, str]], pos: int) -> str:
    current = "?"
    for start, name in spans:
        if start > pos:
            break
        current = name
    return current


# --- Onglets de réglages (Tab.cpp) -----------------------------------------

def extract_tabs() -> list[dict]:
    code = strip_comments(read_source("src/slic3r/GUI/Tab.cpp"))
    spans = function_spans(code)

    events = []  # (pos, kind, payload)
    for m in re.finditer(r'add_options_page\s*\(\s*(L\("(?:[^"\\]|\\.)*"\)|[^,]+),\s*"([^"]*)"', code):
        title = concat_strings(m.group(1)) or m.group(1).strip()
        events.append((m.start(), "page", {"title": title, "icon": m.group(2)}))
    for m in re.finditer(r'new_optgroup\s*\(\s*(L\("(?:[^"\\]|\\.)*"\)|_L\("(?:[^"\\]|\\.)*"\)|"(?:[^"\\]|\\.)*")', code):
        title = concat_strings(m.group(1))
        events.append((m.start(), "optgroup", {"title": title}))
    for m in re.finditer(r'append_single_option_line\s*\(\s*"([^"]+)"', code):
        events.append((m.start(), "option", {"key": m.group(1)}))
    # options ajoutées via une variable (ex: append_single_option_line(option))
    for m in re.finditer(r"append_single_option_line\s*\(\s*([A-Za-z_]\w*)\s*[,)]", code):
        events.append((m.start(), "option_var", {"variable": m.group(1)}))
    # options ajoutées en boucle sur une liste littérale de clés
    # (ex: page Setting Overrides : for (const std::string opt_key : { "filament_...", ... }))
    for m in re.finditer(r"for\s*\(\s*const\s+std::string&?\s+\w+\s*:\s*\{([^}]+)\}", code):
        for key in re.findall(r'"([^"]+)"', m.group(1)):
            events.append((m.start(), "option", {"key": key}))
    events.extend(extract_line_events(code, spans))
    events.sort(key=lambda e: e[0])

    # Ordre d'exécution d'Orca : une page créée dans une fonction auxiliaire
    # (ex. TabFilament::add_filament_overrides_page, définie AVANT build)
    # n'existe à l'exécution qu'au point d'APPEL de cette fonction dans build.
    # Les évènements d'une telle fonction (appel unique) sont ordonnés à la
    # position de l'appel — seuls les types posés (defined_in, line) restent
    # pris à la définition.
    extents = [
        (start, spans[i + 1][0] if i + 1 < len(spans) else len(code), name)
        for i, (start, name) in enumerate(spans)
    ]

    def order_key(e: tuple[int, str, dict]) -> int:
        pos = e[0]
        for start, end, name in extents:
            if start <= pos < end:
                short = name.split("::")[-1]
                if short.startswith("build"):
                    return pos
                calls = [
                    m.start()
                    for m in re.finditer(rf"\b{re.escape(short)}\s*\(\s*\)", code)
                    if not (start <= m.start() < end)
                ]
                return calls[0] if len(calls) == 1 else pos
        return pos

    events.sort(key=order_key)

    pages: list[dict] = []
    current_page = None
    current_group = None
    for pos, kind, payload in events:
        ctx = context_at(spans, pos)
        if kind == "page":
            current_page = {
                "title": payload["title"],
                "icon": payload["icon"],
                "defined_in": ctx,
                "line": line_of(code, pos),
                "sections": [],
            }
            current_group = None
            pages.append(current_page)
        elif kind == "optgroup":
            if current_page is None or context_at(spans, pos) != current_page["defined_in"]:
                # optgroup hors d'une page connue (ex: panneau custom) : page implicite
                current_page = {
                    "title": None,
                    "icon": None,
                    "defined_in": ctx,
                    "line": line_of(code, pos),
                    "sections": [],
                }
                pages.append(current_page)
            current_group = {"title": payload["title"], "options": []}
            current_page["sections"].append(current_group)
        elif kind in ("option", "option_var"):
            if current_group is None:
                continue
            if kind == "option":
                current_group["options"].append(payload["key"])
            else:
                resolved = resolve_option_var(code, spans, pos, payload["variable"])
                current_group["options"].append(resolved or {"dynamic": payload["variable"]})
        elif kind == "line_option":
            # Option d'une `Line` composée (Line{label}.append_option(...)) :
            # rendue comme une ligne d'option à part entière ; l'étiquette de la
            # ligne Orca (« Nozzle », « Cool Plate »…) est conservée dans "lines".
            if current_group is None:
                continue
            key = payload["key"]
            current_group["options"].append(key)
            lines = current_group.setdefault("lines", [])
            line_id = (payload["line_var"], payload["line_pos"])
            if lines and lines[-1].get("_id") == line_id:
                lines[-1]["options"].append(key)
            else:
                entry = {"_id": line_id, "options": [key]}
                if payload.get("label_from"):
                    entry["label_from"] = payload["label_from"]
                else:
                    entry["label"] = payload.get("label") or ""
                lines.append(entry)
    for page in pages:
        for section in page["sections"]:
            for line in section.get("lines", []):
                line.pop("_id", None)
    return pages


# --- Lignes composées de Tab.cpp (FIL-a, lot 47) ----------------------------
# Orca construit une partie des lignes à la main, hors append_single_option_line :
#   Line line = { L("Nozzle"), L("…") };
#   line.append_option(optgroup->get_option("nozzle_temperature_initial_layer", 0));
#   Option opt = optgroup->get_option("chamber_temperature"); line.append_option(opt);
#   append_option_line(optgroup, "machine_max_speed_x", …)   (TabPrinter, libellé = full_label)
#   for (const std::string &axis : axes) append_option_line(optgroup, "machine_max_jerk_" + axis, …)
# Sans elles, « Print temperature », « Bed temperature » et les limites machine
# sortaient vides du layout.

ASSIGN_GET_OPTION_RE = r'\b{var}\s*=\s*\w+(?:->|\.)get_option\(\s*"([^"]+)"'


def _function_extent(spans: list[tuple[int, str]], pos: int, code_len: int) -> tuple[int, int]:
    start, end = 0, code_len
    for s, _name in spans:
        if s > pos:
            end = s
            break
        start = s
    return start, end


def resolve_option_var(code: str, spans: list[tuple[int, str]], pos: int, var: str) -> str | None:
    """Clé de la dernière affectation `var = ...get_option("clé")` avant `pos`,
    dans la même fonction ; None si cette affectation n'est pas littérale."""
    start, _end = _function_extent(spans, pos, len(code))
    last = None
    for m in re.finditer(rf"\b{re.escape(var)}\s*=(?!=)", code[start:pos]):
        last = m
    if last is None:
        return None
    tail = code[start + last.start(): pos]
    mm = re.match(ASSIGN_GET_OPTION_RE.format(var=re.escape(var)), tail)
    return mm.group(1) if mm else None


def extract_line_events(code: str, spans: list[tuple[int, str]]) -> list[tuple[int, str, dict]]:
    events: list[tuple[int, str, dict]] = []
    # Débuts de lignes : Line x = { L("…") / x = { L("…") / Line x = Line{ L("…")
    line_starts: list[tuple[int, str, str]] = []
    for m in re.finditer(
        r'(?:\bLine\s+)?\b(\w+)\s*=\s*(?:Line\s*)?\{\s*(?:_?L\()?\s*"((?:[^"\\]|\\.)*)"', code
    ):
        line_starts.append((m.start(), m.group(1), m.group(2)))

    def line_of_var(var: str, pos: int) -> tuple[int, str]:
        start, _ = _function_extent(spans, pos, len(code))
        best = (pos, "")
        for p, v, label in line_starts:
            if v == var and start <= p < pos:
                best = (p, label)
        return best

    for m in re.finditer(r'\b(\w+)\.append_option\(\s*\w+(?:->|\.)get_option\(\s*"([^"]+)"', code):
        lp, label = line_of_var(m.group(1), m.start())
        events.append((m.start(), "line_option",
                       {"key": m.group(2), "line_var": m.group(1), "line_pos": lp, "label": label}))
    for m in re.finditer(r"\b(\w+)\.append_option\(\s*([A-Za-z_]\w*)\s*\)", code):
        key = resolve_option_var(code, spans, m.start(), m.group(2))
        if key is None:
            continue
        lp, label = line_of_var(m.group(1), m.start())
        events.append((m.start(), "line_option",
                       {"key": key, "line_var": m.group(1), "line_pos": lp, "label": label}))

    # TabPrinter::append_option_line(optgroup, "clé", …) : une ligne par clé,
    # libellé = full_label de l'option.
    for m in re.finditer(r'\bappend_option_line\s*\(\s*\w+\s*,\s*"([^"]+)"\s*[,)]', code):
        events.append((m.start(), "line_option",
                       {"key": m.group(1), "line_var": "append_option_line", "line_pos": m.start(),
                        "label_from": "full_label"}))
    # … et en boucle sur un vecteur littéral :
    # for (const std::string &v : vec) { append_option_line(optgroup, v | "prefixe" + v, …); }
    vectors = {
        m.group(1): re.findall(r'"([^"]+)"', m.group(2))
        for m in re.finditer(r"std::vector<std::string>\s*(\w+)\s*\{([^}]*)\}", code)
    }
    for m in re.finditer(
        r"for\s*\(\s*const\s+std::string\s*&\s*(\w+)\s*:\s*(\w+)\s*\)\s*\{([^{}]*)\}", code
    ):
        var, vec, body = m.group(1), m.group(2), m.group(3)
        if vec not in vectors:
            continue
        call = re.search(
            rf'append_option_line\s*\(\s*\w+\s*,\s*(?:"([^"]*)"\s*\+\s*)?{re.escape(var)}\s*[,)]', body
        )
        if not call:
            continue
        prefix = call.group(1) or ""
        for i, item in enumerate(vectors[vec]):
            pos = m.start() + i  # positions distinctes, ordonnées dans la boucle
            events.append((pos, "line_option",
                           {"key": prefix + item, "line_var": "append_option_line", "line_pos": pos,
                            "label_from": "full_label"}))
    return events


# --- Menus (MainFrame.cpp, GUI_Factories.cpp) ------------------------------

MODIFIER_VARS = {
    "ctrl": "Ctrl+",
    "alt": "Alt+",
    "shift": "Shift+",
    "sep": "",
    "sep_space": "",
    "dots": "…",
}


def resolve_label_expr(expr: str) -> tuple[str | None, str | None]:
    """Résout une expression de libellé wxWidgets → (libellé, raccourci)."""
    # _L_CONTEXT("Top", "Camera View") : seul le premier littéral est affiché,
    # le second est un contexte de traduction (sinon « TopCamera View »).
    expr = re.sub(
        r'_L_CONTEXT\s*\(\s*("(?:[^"\\]|\\.)*")(?:\s*,\s*"(?:[^"\\]|\\.)*")?\s*\)',
        r"\1",
        expr,
    )
    # remplace les variables de modificateur usuelles par leur littéral
    for var, lit in MODIFIER_VARS.items():
        expr = re.sub(rf"\b{var}\b(?!\()", f'"{lit}"', expr)
    s = concat_strings(expr)
    if s is None:
        return None, None
    if "\t" in s:
        label, _, accel = s.partition("\t")
        return label.strip(), accel.strip()
    return s.strip(), None


def extract_menu_items(relpath: str, shortcut_chords: dict[str, str] | None = None) -> list[dict]:
    """Éléments de menu : append_menu_item + append_shortcut_item (main).

    append_shortcut_item(menu, Shortcut::Id, enabled, label, desc, …) : le
    libellé est en 4e argument et l'accélérateur vient de la table des
    raccourcis (Shortcuts.cpp), plus du texte du menu. Les branches
    #ifndef/#else __APPLE__ dupliquent l'appel (icône seule différence) :
    dédupliqué par (menu, Shortcut::Id), la première branche gagne."""
    code = strip_comments(read_source(relpath))
    spans = function_spans(code)
    items = []
    seen_shortcut_items: set[tuple[str, str]] = set()
    for m in re.finditer(
        r"append_(?:menu_(?P<mkind>item|check_item|radio_item)|(?P<skind>shortcut_item))\s*\(", code
    ):
        open_pos = m.end() - 1
        # découpe les arguments de premier niveau
        depth = 0
        args, buf = [], []
        i = open_pos
        while i < len(code):
            c = code[i]
            if c == '"':
                mm = re.compile(r'"(?:[^"\\]|\\.)*"').match(code, i)
                buf.append(mm.group(0))
                i = mm.end()
                continue
            if c in "([{":
                depth += 1
                if depth > 1:
                    buf.append(c)
            elif c in ")]}":
                depth -= 1
                if depth == 0:
                    args.append("".join(buf).strip())
                    break
                buf.append(c)
            elif c == "," and depth == 1:
                args.append("".join(buf).strip())
                buf = []
            else:
                buf.append(c)
            i += 1
        if m.group("skind") is not None:
            # append_shortcut_item(menu, Shortcut::Id, enabled, label, desc, …)
            if len(args) < 5:
                continue
            sm = re.match(r"^Shortcut::(\w+)$", args[1].strip())
            if not sm:
                continue
            dedupe = (args[0], sm.group(1))
            if dedupe in seen_shortcut_items:
                continue  # branche #else __APPLE__ du même appel
            seen_shortcut_items.add(dedupe)
            label, accel = resolve_label_expr(args[3])
            desc = concat_strings(args[4])
            accel = accel or (shortcut_chords or {}).get(sm.group(1))
        else:
            if len(args) < 3:
                continue
            label, accel = resolve_label_expr(args[2])
            desc = concat_strings(args[3]) if len(args) > 3 else None
        if label is None:
            continue
        items.append({
            "menu_variable": args[0],
            "kind": m.group("skind") or m.group("mkind"),
            "label": label,
            "shortcut": accel,
            "description": desc or None,
            "defined_in": context_at(spans, m.start()),
            "line": line_of(code, m.start()),
        })
    return items


def extract_submenus(relpath: str) -> list[dict]:
    code = strip_comments(read_source(relpath))
    subs = []
    for m in re.finditer(r"append_submenu\s*\(\s*(\w+)\s*,\s*(\w+)\s*,\s*wx[\w:]+\s*,\s*([^,]+),", code):
        title = concat_strings(m.group(3))
        if title:
            subs.append({"parent_variable": m.group(1), "submenu_variable": m.group(2), "title": title})
    # m_menubar->Append(menu, _L("&Titre"))
    for m in re.finditer(r"(?:m_menubar|menubar)->Append\s*\(\s*(\w+)\s*,\s*([^)]+)\)", code):
        title = concat_strings(m.group(2))
        if title:
            subs.append({"menubar_variable": m.group(1), "title": title})
    return subs


# --- Barres d'outils 3D (GLCanvas3D.cpp) -----------------------------------

def extract_toolbars() -> dict[str, list[dict]]:
    code = strip_comments(read_source("src/slic3r/GUI/GLCanvas3D.cpp"))
    spans = function_spans(code)
    toolbars: dict[str, list[dict]] = {}
    matches = list(re.finditer(r'\b(\w*item)\.name\s*=\s*"([^"]+)"\s*;', code))
    for i, m in enumerate(matches):
        end = matches[i + 1].start() if i + 1 < len(matches) else m.start() + 4000
        block = code[m.start():end]
        entry: dict = {"name": m.group(2), "line": line_of(code, m.start())}
        mm = re.search(rf"{re.escape(m.group(1))}\.icon_filename\s*=\s*(.*?);", block, re.S)
        if mm:
            icon = concat_strings(mm.group(1))
            if icon:
                entry["icon"] = icon
        mm = re.search(rf"{re.escape(m.group(1))}\.tooltip\s*=\s*(.*?);\n", block, re.S)
        if mm:
            tooltip = concat_strings(mm.group(1))
            if tooltip:
                entry["tooltip"] = tooltip
        func = context_at(spans, m.start())
        toolbars.setdefault(func, []).append(entry)
    return toolbars


# --- Gizmos (GLGizmosManager.cpp) -------------------------------------------

def extract_gizmos() -> list[dict]:
    code = strip_comments(read_source("src/slic3r/GUI/Gizmos/GLGizmosManager.cpp"))
    gizmos = []
    for m in re.finditer(r"m_gizmos\.emplace_back\(\s*new\s+(GLGizmo\w+)\s*\(", code):
        open_pos = m.end() - 1
        close = code.find(");", open_pos)
        args = code[open_pos:close]
        entry: dict = {"class": m.group(1), "line": line_of(code, m.start())}
        icons = re.findall(r'"([^"]+\.svg)"', args)
        if icons:
            entry["icon"] = icons[-1]  # variante claire (la dernière du ternaire dark/light)
        tm = re.search(r"EType::(\w+)", args)
        if tm:
            entry["type"] = tm.group(1)
        gizmos.append(entry)
    return gizmos


# --- Raccourcis clavier (Shortcuts.cpp depuis main aefb6b3d) ----------------
# Main a remplacé les listes statiques de KBShortcutsDialog::fill_shortcuts
# par un registre data-driven : shortcut_table (SHORTCUT/REPEATING/STEPPING)
# + section_table (titre de chaque section). Les raccourcis par défaut y
# sont exprimés en accords (constants WXK_/modificateurs), les branches
# #ifdef __APPLE__ étant écartées (dernière définition = #else).

SHORTCUTS_SOURCE = "src/slic3r/GUI/Shortcuts.cpp"

# Codes wxWidgets rencontrés dans shortcut_table → libellé affiché.
WXK_KEYS = {
    "WXK_TAB": "Tab", "WXK_RETURN": "Return", "WXK_BACK": "Backspace",
    "WXK_DELETE": "Del", "WXK_LEFT": "Left", "WXK_RIGHT": "Right",
    "WXK_UP": "Up", "WXK_DOWN": "Down", "WXK_PAGEUP": "PageUp",
    "WXK_PAGEDOWN": "PageDown", "WXK_HOME": "Home", "WXK_END": "End",
    "WXK_F5": "F5", "WXK_SPACE": "Space", "WXK_ESCAPE": "Esc",
}
SHORTCUT_MODIFIERS = {"CTRL": "Ctrl", "SHIFT": "Shift", "CTRL_SHIFT": "Ctrl+Shift", "ALT": "Alt"}

# constexpr KeyChord NAME { … };  (deux branches #ifdef, on garde la dernière)
CHORD_CONST_RE = re.compile(r"constexpr\s+KeyChord\s+(\w+)\s*\{([^}]*)\}")
# SHORTCUT(Id, "config_key", L("Name"), CONTEXTS, chord)
SHORTCUT_ROW_RE = re.compile(
    r'\b(?:SHORTCUT|REPEATING|STEPPING)\s*\(\s*(\w+)\s*,\s*"([^"]+)"\s*,'
    r'\s*(L\(\s*"((?:[^"\\]|\\.)*)"(?:\s*"(?:[^"\\]|\\.)*"\s*)*\))\s*,'
    r'\s*[A-Z_]+(?:\s*\|\s*[A-Z_]+)*\s*,\s*(\{[^{}]*\}|[A-Z_][A-Z0-9_]*)\s*\)'
)
SECTION_ROW_RE = re.compile(r'\{\s*Shortcut::(\w+)\s*,\s*L\(\s*"((?:[^"\\]|\\.)*)"\s*\)\s*\}')

# { Shortcut::FirstId, L("Titre") } — bornes de sections de section_table.
SECTION_ROW_RE = re.compile(r'\{\s*Shortcut::(\w+)\s*,\s*L\(\s*"((?:[^"\\]|\\.)*)"\s*\)\s*\}')


def render_chord(expr: str, chord_consts: dict[str, str]) -> str:
    """{ 'N', CTRL } / DELETE_CHORD / {} → « Ctrl+N » / « Del » / « »."""
    expr = expr.strip()
    if not expr.startswith("{"):
        return chord_consts.get(expr, expr)
    inner = expr.strip("{}").strip()
    if not inner:
        return ""  # pas de touche par défaut (GoTo*)
    mods: list[str] = []
    key = ""
    for part in (p.strip() for p in inner.split(",") if p.strip()):
        if part in SHORTCUT_MODIFIERS:
            mods.append(SHORTCUT_MODIFIERS[part])
        elif part.startswith("'") and part.endswith("'") and len(part) >= 3:
            key = part[1:-1]
        elif part in WXK_KEYS:
            key = WXK_KEYS[part]
        else:
            key = part
    return "+".join(mods + ([key] if key else []))


def extract_shortcuts() -> tuple[list[dict], dict[str, str]]:
    """Raccourcis par défaut → (groupes, {Id → accord affiché}).

    main (aefb6b3d) : registre data-driven de Shortcuts.cpp. Repli 2.4.1 :
    listes statiques de KBShortcutsDialog::fill_shortcuts (plus d'accords
    joints aux menus)."""
    if (ORCA_ROOT / SHORTCUTS_SOURCE).exists():
        return extract_shortcuts_main()
    return extract_shortcuts_legacy(), {}


def extract_shortcuts_legacy() -> list[dict]:
    """Raccourcis de la 2.4.1 : m_full_shortcuts + listes Shortcuts du
    KBShortcutsDialog (gardé pour les sources pré-main, ex. devShell 2.4.1)."""
    code = strip_comments(read_source("src/slic3r/GUI/KBShortcutsDialog.cpp"))
    body_m = re.search(r"void\s+KBShortcutsDialog::fill_shortcuts\s*\(\)\s*\{", code)
    if not body_m:
        return []
    body = code[body_m.end():]

    # groupes : m_full_shortcuts.push_back({{_L("Titre"), ""}, variable});
    group_titles: dict[str, str] = {}
    group_order: list[str] = []
    for m in re.finditer(r'm_full_shortcuts\.push_back\(\s*\{\s*\{\s*_L\("([^"]+)"\)\s*,\s*"[^"]*"\s*\}\s*,\s*(\w+)\s*\}\s*\)', body):
        group_titles[m.group(2)] = m.group(1)
        group_order.append(m.group(2))

    # listes : Shortcuts xxx = { {expr, L("desc")}, ... };
    groups = []
    for m in re.finditer(r"Shortcuts\s+(\w+)\s*=\s*\{", body):
        var = m.group(1)
        depth, i = 1, m.end()
        while i < len(body) and depth:
            c = body[i]
            if c == '"':
                mm = re.compile(r'"(?:[^"\\]|\\.)*"').match(body, i)
                i = mm.end()
                continue
            if c == "{":
                depth += 1
            elif c == "}":
                depth -= 1
            i += 1
        block = body[m.end(): i - 1]
        entries = []
        for em in re.finditer(r"\{((?:[^{}\"]|\"(?:[^\"\\]|\\.)*\")+?),\s*L\(\s*(\"(?:[^\"\\]|\\.)*\"(?:\s*\"(?:[^\"\\]|\\.)*\")*)\s*\)\s*\}", block):
            key_expr = em.group(1)
            for var_name, lit in MODIFIER_VARS.items():
                key_expr = re.sub(rf"\b{var_name}\b", f'"{lit}"', key_expr)
            key = concat_strings(key_expr)
            desc = concat_strings(em.group(2))
            if key is not None and desc:
                entries.append({"keys": key, "action": desc})
        groups.append({
            "group": group_titles.get(var, var),
            "variable": var,
            "shortcuts": entries,
        })
    order = {v: i for i, v in enumerate(group_order)}
    groups.sort(key=lambda g: order.get(g["variable"], len(order)))
    return groups


def extract_shortcuts_main() -> tuple[list[dict], dict[str, str]]:
    """Raccourcis par défaut de main → (groupes, {Id → accord affiché})."""
    code = strip_comments(read_source(SHORTCUTS_SOURCE))

    # Constantes d'accords : la DERNIÈRE définition gagne (branche #else,
    # hors __Apple__).
    chord_consts: dict[str, str] = {}
    for m in CHORD_CONST_RE.finditer(code):
        chord_consts[m.group(1)] = render_chord("{" + m.group(2) + "}", {})

    # Bornes des sections (section_table : premier raccourci de chaque titre).
    sections: list[tuple[str, str]] = []
    st = re.search(r"section_table\s*=\s*\{\{(.*?)\}\}", code, re.S)
    if st:
        for m in SECTION_ROW_RE.finditer(st.group(1)):
            sections.append((m.group(1), m.group(2)))
    section_at = {sid: title for sid, title in sections}

    groups: list[dict] = []
    current: dict | None = None
    chords: dict[str, str] = {}
    for m in SHORTCUT_ROW_RE.finditer(code):
        # groupes : 1 id, 2 config_key, 3 L("…") complet, 4 premier littéral, 5 accord
        sid, config_key, name_expr, _first_literal, chord_expr = m.groups()
        name = concat_strings(name_expr)
        if not name:
            continue
        keys = render_chord(chord_expr, chord_consts)
        chords[sid] = keys
        if sid in section_at:
            current = {"group": section_at[sid], "shortcuts": []}
            groups.append(current)
        if current is None:
            current = {"group": "(hors section)", "shortcuts": []}
            groups.append(current)
        current["shortcuts"].append({
            "keys": keys,
            "action": name,
            "id": sid,
            "config_key": config_key,
        })
    return groups, chords


def main() -> None:
    # Raccourcis d'abord : la table Shortcuts.cpp fournit les accélérateurs des
    # entrées de menu déclarées via append_shortcut_item.
    shortcuts, shortcut_chords = extract_shortcuts()
    tabs = extract_tabs()
    main_menu_items = extract_menu_items("src/slic3r/GUI/MainFrame.cpp", shortcut_chords)
    main_menu_structure = extract_submenus("src/slic3r/GUI/MainFrame.cpp")
    context_menu_items = extract_menu_items("src/slic3r/GUI/GUI_Factories.cpp", shortcut_chords)
    toolbars = extract_toolbars()
    gizmos = extract_gizmos()

    n_options = sum(len(s["options"]) for p in tabs for s in p["sections"])
    n_shortcuts = sum(len(g["shortcuts"]) for g in shortcuts)

    data = {
        "generated_by": "audit/extract_ui_inventory.py",
        "sources": [
            "src/slic3r/GUI/Tab.cpp",
            "src/slic3r/GUI/MainFrame.cpp",
            "src/slic3r/GUI/GUI_Factories.cpp",
            "src/slic3r/GUI/GLCanvas3D.cpp",
            "src/slic3r/GUI/Gizmos/GLGizmosManager.cpp",
            "src/slic3r/GUI/Shortcuts.cpp",
        ],
        "summary": {
            "settings_pages": len(tabs),
            "settings_sections": sum(len(p["sections"]) for p in tabs),
            "settings_option_lines": n_options,
            "main_menu_items": len(main_menu_items),
            "context_menu_items": len(context_menu_items),
            "toolbar_items": sum(len(v) for v in toolbars.values()),
            "gizmos": len(gizmos),
            "keyboard_shortcuts": n_shortcuts,
        },
        "settings_tabs": tabs,
        "main_menus": {"structure": main_menu_structure, "items": main_menu_items},
        "plater_context_menus": context_menu_items,
        "plater_toolbars": toolbars,
        "plater_gizmos": gizmos,
        "keyboard_shortcuts": shortcuts,
    }
    out = write_json("ui_inventory.json", data)
    print(f"{out}: {data['summary']}")


if __name__ == "__main__":
    main()
