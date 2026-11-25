"""Helpers partagés par les scripts d'audit d'OrcaSlicer.

Tous les scripts sont ré-exécutables : ils lisent vendor/OrcaSlicer et
écrivent des JSON déterministes dans audit/.
"""

from __future__ import annotations

import json
import os
import re
from pathlib import Path

AUDIT_DIR = Path(__file__).resolve().parent
REPO_ROOT = AUDIT_DIR.parent
# La référence de parité est Orca main (vendor/OrcaSlicer @ aefb6b3d) ; la CI
# l'épingle via l'attr nix orca-main-src (ORCA_SRC). Le repli sans ORCA_SRC
# reste le submodule vendor/, qui suit main.
ORCA_ROOT = Path(os.environ.get("ORCA_SRC", REPO_ROOT / "vendor" / "OrcaSlicer"))

_STRING_RE = re.compile(r'"((?:[^"\\]|\\.)*)"')

_ESCAPES = {
    "n": "\n", "t": "\t", "r": "\r", '"': '"', "\\": "\\", "'": "'", "0": "\0",
}


def read_source(relpath: str) -> str:
    return (ORCA_ROOT / relpath).read_text(encoding="utf-8", errors="replace")


def unescape_c(s: str) -> str:
    out = []
    i = 0
    while i < len(s):
        c = s[i]
        if c == "\\" and i + 1 < len(s):
            # \uXXXX / \UXXXXXXXX (littéraux u8"\u2103" : « ℃ »).
            width = {"u": 4, "U": 8}.get(s[i + 1])
            if width and re.fullmatch(r"[0-9A-Fa-f]{%d}" % width, s[i + 2 : i + 2 + width]):
                out.append(chr(int(s[i + 2 : i + 2 + width], 16)))
                i += 2 + width
                continue
            out.append(_ESCAPES.get(s[i + 1], "\\" + s[i + 1]))
            i += 2
        else:
            out.append(c)
            i += 1
    return "".join(out)


def concat_strings(expr: str) -> str | None:
    """Concatène tous les littéraux C d'une expression (gère L("a" "b"), _L(..) + "c")."""
    parts = _STRING_RE.findall(expr)
    if not parts:
        return None
    return unescape_c("".join(parts))


_COMMENT_RE = re.compile(
    r'//[^\n]*|/\*.*?\*/|"(?:[^"\\]|\\.)*"|\'(?:[^\'\\]|\\.)*\'', re.S
)


def strip_comments(code: str) -> str:
    """Supprime les commentaires C/C++ en préservant les littéraux et les numéros de ligne."""

    def repl(m: re.Match) -> str:
        s = m.group(0)
        if s.startswith("/"):
            return "\n" * s.count("\n")
        return s

    return _COMMENT_RE.sub(repl, code)


def find_matching_paren(text: str, open_pos: int, open_ch: str = "(", close_ch: str = ")") -> int:
    """Retourne l'index de la parenthèse fermante appariée (open_pos pointe sur l'ouvrante).

    Ignore les parenthèses situées dans des littéraux de chaîne.
    """
    depth = 0
    i = open_pos
    n = len(text)
    char_re = re.compile(r"'(?:[^'\\]|\\.)*'")
    while i < n:
        c = text[i]
        if c == '"':
            m = _STRING_RE.match(text, i)
            if m:
                i = m.end()
                continue
            i += 1
            continue
        if c == "'":
            m = char_re.match(text, i)
            if m:
                i = m.end()
                continue
            i += 1
            continue
        if c == open_ch:
            depth += 1
        elif c == close_ch:
            depth -= 1
            if depth == 0:
                return i
        i += 1
    raise ValueError(f"unbalanced {open_ch}{close_ch} at {open_pos}")


def line_of(text: str, pos: int) -> int:
    return text.count("\n", 0, pos) + 1


def write_json(filename: str, data: dict) -> Path:
    out = AUDIT_DIR / filename
    out.write_text(
        json.dumps(data, indent=2, ensure_ascii=False, sort_keys=False) + "\n",
        encoding="utf-8",
    )
    return out
