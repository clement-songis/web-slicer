#!/usr/bin/env python3
"""Hook PreToolUse : empêche l'agent de toucher aux juges (tests gelés, harness, état).

Modes (variable HARNESS_MODE, posée par la boucle) :
  impl/review (défaut) : écriture interdite dans tests/oracle/, harness/, audit/*.json, .claude/settings.json
  tests                : écriture autorisée UNIQUEMENT dans tests/oracle/scenarios/<id>.spec.ts du lot
HARNESS_UNLOCK=1 désactive le garde (session humaine qui fait évoluer le harness).
Code 2 = refus ; le message sur stderr est renvoyé à l'agent.
"""
from __future__ import annotations

import json
import os
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
STATE = Path(os.environ.get("HARNESS_STATE", Path.home() / ".local/state/web-slicer-harness")).resolve()
PROTECTED = ("tests/oracle/", "harness/", ".claude/settings.json", "audit/")
WRITE_TOOLS = {"Edit", "Write", "MultiEdit", "NotebookEdit"}
READ_TOOLS = {"Read", "Grep", "Glob"}
BASH_PROTECTED = re.compile(r"tests/oracle|harness/|\.claude/settings|audit/[\w.-]*\.json")
BASH_WRITE = re.compile(
    r"(?<![<>=])>{1,2}(?!&)|\btee\b|\bsed\s+(-\w*\s+)*-i|\bperl\s+-\w*i|\brm\b|\bmv\b|\bcp\b|\btruncate\b|\bdd\b|"
    r"\bchmod\b|\bln\b|\bpython3?\b(?!\s+harness/verify\.py)|\bnode\s+-e|\bbun\s+-e|\bgit\s+(checkout|restore|reset|stash|clean|apply|am|mv|rm)\b|"
    r"\btar\b|\bunzip\b|\binstall\b"
)
BASH_FORBIDDEN = re.compile(
    r"web-slicer-harness|HARNESS_STATE|HARNESS_UNLOCK|HARNESS_MODE=|\b(freeze|save_features|restore_frozen|tamper_report)\b|"
    r"--no-verify|\bgit\s+push\b|\bgit\s+commit\b.*--amend|\bgit\s+rebase\b|\bgit\s+filter-branch\b"
)
SAFE_BASH = re.compile(r"^\s*(python3 harness/verify\.py --check( --mode tests)?( [\w.\-]+)+|harness/stack\.sh (up|logs))\s*$")


def deny(msg: str) -> None:
    print(f"[harness] REFUSÉ : {msg}", file=sys.stderr)
    sys.exit(2)


def rel(path: str, cwd: str) -> tuple[str, Path]:
    p = (Path(cwd) / path).resolve() if not os.path.isabs(path) else Path(path).resolve()
    try:
        return p.relative_to(REPO).as_posix(), p
    except ValueError:
        return "", p


def main() -> None:
    if os.environ.get("HARNESS_UNLOCK") == "1":
        return
    data = json.load(sys.stdin)
    tool, ti = data.get("tool_name", ""), data.get("tool_input", {}) or {}
    cwd = data.get("cwd") or str(REPO)
    mode = os.environ.get("HARNESS_MODE", "impl")
    ids = os.environ.get("HARNESS_FEATURE", "").split()

    if tool in WRITE_TOOLS | READ_TOOLS:
        target = ti.get("file_path") or ti.get("notebook_path") or ti.get("path") or ""
        if not target:
            return
        r, absp = rel(target, cwd)
        if str(absp).startswith(str(STATE)):
            deny("l'état du harness est hors d'atteinte de l'agent")
        if tool in READ_TOOLS:
            return
        if mode == "tests":
            allowed = {f"tests/oracle/scenarios/{i}.spec.ts" for i in ids}
            if r not in allowed:
                deny(f"en mode auteur de tests, seuls ces fichiers sont modifiables : {sorted(allowed)}")
            return
        if r.startswith(PROTECTED) or r == ".claude/settings.json":
            deny(f"{r} est un juge gelé (tests oracle / harness / audit). Si un test te semble faux, "
                 "explique-le dans ta NOTE finale (TEST_SUSPECT: ...) au lieu de le modifier.")
        return

    if tool == "Bash":
        cmd = ti.get("command", "")
        if BASH_FORBIDDEN.search(cmd):
            deny("commande interdite par le harness (état, gel des tests, réécriture d'historique, push)")
        if SAFE_BASH.match(cmd):
            return
        if mode == "tests" and re.search(r"\b(frontend|backend|engine)/", cmd) and BASH_WRITE.search(cmd):
            deny("en mode auteur de tests, le code de l'application est en lecture seule")
        if BASH_PROTECTED.search(cmd) and BASH_WRITE.search(cmd):
            deny("écriture (ou commande non sûre) sur un chemin gelé : tests/oracle, harness/, audit/, .claude/settings")


if __name__ == "__main__":
    main()
