#!/usr/bin/env python3
"""Extrait l'API publique de libslic3r consommée par la GUI.

Parse les headers du pipeline de slicing (Model, Print, PrintObject, slicing,
maillages, presets, G-code) et liste les classes + méthodes publiques.
Chaque classe/méthode est annotée avec le nombre de références trouvées dans
src/slic3r (le code GUI), pour distinguer l'API réellement consommée.

Sortie : audit/engine_api.json
"""

from __future__ import annotations

import re
from collections import Counter
from pathlib import Path

from common import ORCA_ROOT, line_of, read_source, strip_comments, write_json

# Headers du moteur exposant l'API utilisée par la GUI (pipeline de slicing).
HEADERS = [
    "src/libslic3r/Model.hpp",
    "src/libslic3r/Print.hpp",
    "src/libslic3r/PrintBase.hpp",
    "src/libslic3r/Slicing.hpp",
    "src/libslic3r/SlicingAdaptive.hpp",
    "src/libslic3r/TriangleMesh.hpp",
    "src/libslic3r/TriangleMeshSlicer.hpp",
    "src/libslic3r/PrintConfig.hpp",
    "src/libslic3r/Preset.hpp",
    "src/libslic3r/PresetBundle.hpp",
    "src/libslic3r/GCode.hpp",
    "src/libslic3r/GCode/GCodeProcessor.hpp",
    "src/libslic3r/GCode/ThumbnailData.hpp",
    "src/libslic3r/BuildVolume.hpp",
]

GUI_SRC_DIR = ORCA_ROOT / "src" / "slic3r"

CLASS_RE = re.compile(r"\b(class|struct)\s+([A-Za-z_]\w*)\s*(final\s*)?(?::\s*([^{;]+?))?\s*\{")

KEYWORD_BLACKLIST = {
    "if", "for", "while", "switch", "return", "sizeof", "catch", "new",
    "delete", "throw", "assert", "static_assert", "alignas", "decltype",
}


def find_brace_close(text: str, open_pos: int) -> int:
    depth = 0
    i = open_pos
    n = len(text)
    while i < n:
        c = text[i]
        if c == '"':
            m = re.compile(r'"(?:[^"\\]|\\.)*"').match(text, i)
            if m:
                i = m.end()
                continue
        if c == "'":
            m = re.compile(r"'(?:[^'\\]|\\.)*'").match(text, i)
            if m:
                i = m.end()
                continue
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return i
        i += 1
    raise ValueError("unbalanced braces")


def split_declarations(body: str):
    """Découpe le corps d'une classe en déclarations de premier niveau.

    Retourne des tuples (texte_déclaration, avait_un_corps_inline, position).
    """
    decls = []
    buf_start = 0
    i = 0
    n = len(body)
    while i < n:
        c = body[i]
        if c == '"':
            m = re.compile(r'"(?:[^"\\]|\\.)*"').match(body, i)
            i = m.end() if m else i + 1
            continue
        if c == "'":
            m = re.compile(r"'(?:[^'\\]|\\.)*'").match(body, i)
            i = m.end() if m else i + 1
            continue
        if c == "{":
            close = find_brace_close(body, i)
            decl = body[buf_start:i]
            # « = default/delete » et corps inline terminent la déclaration,
            # sauf initialisation de membre par accolades (rare en API) — accepté.
            tail = body[close + 1 :]
            j = close + 1
            if tail.lstrip().startswith(";"):
                j = close + 1 + tail.index(";") + 1
            decls.append((decl, True, buf_start))
            buf_start = j
            i = j
            continue
        if c == ";":
            decls.append((body[buf_start:i], False, buf_start))
            buf_start = i + 1
        i += 1
    return decls


METHOD_RE = re.compile(
    r"^(?P<prefix>(?:template\s*<[^;{]*?>\s*)?(?:(?:virtual|static|inline|constexpr|explicit|friend|mutable)\s+)*)"
    r"(?P<ret>[\w:<>,&*\s~()\[\]\.]*?)"
    r"(?P<name>operator\s*(?:[^\s(]+|\(\s*\))|~?[A-Za-z_]\w*)\s*$",
    re.S,
)


def parse_method(decl: str) -> dict | None:
    decl = decl.strip()
    if not decl or "(" not in decl:
        return None
    first_word = re.match(r"\w+", decl)
    if decl.startswith(("typedef", "using ", "enum ", "#")):
        return None
    # position de la parenthèse des paramètres : première '(' dont le préfixe
    # matche un nom de fonction
    for m in re.finditer(r"\(", decl):
        head = decl[: m.start()]
        hm = METHOD_RE.match(head)
        if not hm:
            continue
        name = re.sub(r"\s+", " ", hm.group("name")).strip()
        if name in KEYWORD_BLACKLIST:
            continue
        try:
            from common import find_matching_paren
            close = find_matching_paren(decl, m.start())
        except ValueError:
            return None
        params = re.sub(r"\s+", " ", decl[m.start() + 1 : close]).strip()
        suffix = decl[close + 1 :]
        ret = re.sub(r"\s+", " ", hm.group("ret")).strip()
        prefix = hm.group("prefix") or ""
        if "friend" in prefix:
            return None
        entry: dict = {"name": name, "params": params}
        if ret:
            entry["returns"] = ret
        quals = []
        if "static" in prefix:
            quals.append("static")
        if "virtual" in prefix:
            quals.append("virtual")
        if re.search(r"^\s*const\b", suffix):
            quals.append("const")
        if "override" in suffix:
            quals.append("override")
        if re.search(r"=\s*0\s*$", suffix.strip()):
            quals.append("pure")
        if re.search(r"=\s*delete", suffix):
            quals.append("deleted")
        if quals:
            entry["qualifiers"] = quals
        if name.startswith("~"):
            entry["kind"] = "destructor"
        elif ret == "" and "operator" not in name and not name.startswith("~"):
            entry["kind"] = "constructor"
        return entry
    return None


def parse_classes(code: str, min_line: int = 0) -> list[dict]:
    classes = []
    for m in CLASS_RE.finditer(code):
        kind, name, bases = m.group(1), m.group(2), m.group(4)
        # ignore les forward declarations ("class X;") — le regex exige '{'
        # ignore "enum class"
        before = code[max(0, m.start() - 8) : m.start()]
        if re.search(r"enum\s+$", before):
            continue
        open_brace = code.find("{", m.end() - 1)
        try:
            close = find_brace_close(code, open_brace)
        except ValueError:
            continue
        body = code[open_brace + 1 : close]

        access = "private" if kind == "class" else "public"
        methods = []
        for decl, had_body, pos in split_declarations(body):
            # met à jour l'état d'accès selon les spécificateurs traversés
            segments = re.split(r"\b(public|protected|private)\s*:", decl)
            current_text = segments[0]
            for k in range(1, len(segments), 2):
                access = segments[k]
                current_text = segments[k + 1]
            if access != "public":
                continue
            # saute les classes imbriquées (déjà remontées par CLASS_RE global)
            if re.match(r"\s*(class|struct)\b", current_text) and had_body:
                continue
            entry = parse_method(current_text)
            if entry:
                methods.append(entry)
        classes.append({
            "name": name,
            "kind": kind,
            "bases": re.sub(r"\s+", " ", bases).strip() if bases else None,
            "line": line_of(code, m.start()),
            "public_methods": methods,
        })
    return classes


def build_gui_usage_counters() -> tuple[Counter, Counter]:
    """Compteurs d'identifiants et d'appels de méthode dans src/slic3r (GUI)."""
    idents: Counter = Counter()
    calls: Counter = Counter()
    for path in sorted(GUI_SRC_DIR.rglob("*")):
        if path.suffix not in (".cpp", ".hpp", ".h", ".cc"):
            continue
        if "libslic3r" in path.parts:
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        idents.update(re.findall(r"\b[A-Za-z_]\w*\b", text))
        calls.update(re.findall(r"[.>:]\s*(~?[A-Za-z_]\w*)\s*\(", text))
    return idents, calls


def main() -> None:
    idents, calls = build_gui_usage_counters()

    headers_out = []
    total_classes = 0
    total_methods = 0
    for rel in HEADERS:
        if not (ORCA_ROOT / rel).exists():
            headers_out.append({"header": rel, "error": "fichier introuvable"})
            continue
        code = strip_comments(read_source(rel))
        classes = parse_classes(code)
        for cls in classes:
            cls["gui_references"] = idents.get(cls["name"], 0)
            cls["used_by_gui"] = cls["gui_references"] > 0
            for meth in cls["public_methods"]:
                meth["gui_call_sites"] = calls.get(meth["name"], 0)
        total_classes += len(classes)
        total_methods += sum(len(c["public_methods"]) for c in classes)
        headers_out.append({"header": rel, "classes": classes})

    data = {
        "generated_by": "audit/extract_engine_api.py",
        "note": (
            "gui_references = occurrences du nom de classe dans src/slic3r (hors libslic3r) ; "
            "gui_call_sites = occurrences de « .méthode( », « ->méthode( » ou « ::méthode( » "
            "dans la GUI (approximation : les homonymes sont comptés ensemble)."
        ),
        "summary": {
            "headers": len(HEADERS),
            "classes": total_classes,
            "public_methods": total_methods,
        },
        "headers": headers_out,
    }
    out = write_json("engine_api.json", data)
    print(f"{out}: {data['summary']}")


if __name__ == "__main__":
    main()
