#!/usr/bin/env python3
"""Choisit le prochain lot de travail et rend son contexte (court) pour l'agent.

Sortie JSON sur stdout : {"mode": impl|tests|review, "ids": [...], "model": ..., "context": "..."}
Code de sortie 3 : plus rien d'exécutable (tout est validé, bloqué ou en attente de réf.).

Usage : next_feature.py [--session N]
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from lib import ORACLE, VENDOR, load_features, locked, save_features  # noqa: E402

BATCH = {"param": int(os.environ.get("HARNESS_BATCH_PARAMS", "6")), "golden": 6, "shortcut": 4}
TESTS_BATCH = {"shortcut": 4, "menu": 2, "ctx": 2, "param": 6}
REVIEW_EVERY = int(os.environ.get("HARNESS_REVIEW_EVERY", "8"))
MODELS = {
    "impl": os.environ.get("HARNESS_MODEL_IMPL", "sonnet"),
    "tests": os.environ.get("HARNESS_MODEL_TESTS", "opus"),
    "review": os.environ.get("HARNESS_MODEL_REVIEW", "opus"),
}


def ref_path(fid: str) -> Path | None:
    if fid.startswith("param."):
        return ORACLE / "refs" / "param" / f"{fid[6:]}.gcode"
    if fid.startswith("golden."):
        return ORACLE / "refs" / "golden" / f"{fid[7:]}.gcode"
    return None


def promote_refs(feats: dict) -> bool:
    """needs_ref → red quand la référence Orca existe ; → needs_review si Orca a échoué."""
    changed = False
    for fid, f in feats.items():
        if f["status"] != "needs_ref":
            continue
        p = ref_path(fid)
        if p and p.exists():
            f["status"], changed = "red", True
        elif p and p.with_suffix(".error").exists():
            f["status"] = "needs_review"
            f["reason"] = "orca-slicer CLI a refusé la valeur de test : " + p.with_suffix(".error").read_text()[:300]
            changed = True
    return changed


def deps_ok(f: dict, feats: dict) -> bool:
    return all(feats.get(d, {}).get("status") in ("passing", "excluded") for d in f.get("deps", []))


def grep_pointers(fid: str, f: dict) -> list[str]:
    if not fid.startswith("param.") or not (VENDOR / "src").exists():
        return []
    key = fid[6:]
    tool = shutil.which("rg")
    cmd = ([tool, "-l", "-w", "--type", "cpp", key, "src/libslic3r"] if tool
           else ["grep", "-rlw", "--include=*.cpp", key, "src/libslic3r"])
    try:
        out = subprocess.run(cmd, cwd=VENDOR, capture_output=True, text=True, timeout=20).stdout.split()
    except Exception:
        return []
    return [o for o in out if not o.endswith(("PrintConfig.cpp", "Preset.cpp"))][:6]


def render(mode: str, ids: list[str], feats: dict) -> str:
    lines = [f"# Lot ({mode}) — {len(ids)} fonctionnalité(s)", ""]
    for fid in ids:
        f = feats[fid]
        lines += [f"## `{fid}` — {f['title']}", f"- Test : `{f['test']}` (tag `@F:{fid}`)"]
        orca = f.get("orca", []) + [f"src/libslic3r/{p.split('/')[-1]}" for p in grep_pointers(fid, f)]
        if orca:
            lines.append("- Sources Orca (vendor/OrcaSlicer/) : " + ", ".join(f"`{o}`" for o in orca))
        if f.get("attempts"):
            lines.append(f"- Tentatives précédentes : {f['attempts']}")
        if f.get("last_note"):
            lines.append(f"- Note de la session précédente : {f['last_note'][:600]}")
        if f.get("last_error"):
            lines += ["- Dernier échec :", "```", f["last_error"][:1500], "```"]
        if mode == "tests" and fid.startswith("param."):
            lines.append(f"- À écrire : `tests/oracle/params/requires/{fid[6:]}.json` = "
                         '`{"requires": {"<clé>": "<valeur Orca>"}}` : les réglages à activer pour que le champ '
                         "soit éditable chez Orca (voir `src/slic3r/GUI/ConfigManipulation.cpp`, toggle_field/toggle_line).")
        if mode == "review" and f.get("history"):
            lines.append("- Historique : " + json.dumps(f["history"][-4:], ensure_ascii=False)[:1500])
        lines.append("")
    return "\n".join(lines)


def pick(feats: dict, session: int) -> tuple[str, list[str]] | None:
    order = sorted(feats, key=lambda k: (feats[k].get("priority", 99), k))
    if session and session % REVIEW_EVERY == 0:
        for fid in order:
            f = feats[fid]
            if f["status"] == "blocked" and not f.get("reviewed"):
                return "review", [fid]
    for fid in order:
        f = feats[fid]
        if f["status"] not in ("needs_test", "red") or not deps_ok(f, feats):
            continue
        mode = "tests" if f["status"] == "needs_test" else "impl"
        limit = (TESTS_BATCH if mode == "tests" else BATCH).get(f["kind"], 1)
        batch = [fid]
        if f.get("batch") and limit > 1:
            for other in order:
                g = feats[other]
                if (other != fid and g.get("batch") == f["batch"] and g["status"] == f["status"]
                        and deps_ok(g, feats)):
                    batch.append(other)
                if len(batch) >= limit:
                    break
        return mode, batch
    return None


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--session", type=int, default=0)
    args = ap.parse_args()
    with locked():
        data = load_features()
        feats = data["features"]
        if promote_refs(feats):
            save_features(data)
        choice = pick(feats, args.session)
    if not choice:
        from collections import Counter
        print(json.dumps({"mode": None, "summary": Counter(f["status"] for f in feats.values())}))
        return 3
    mode, ids = choice
    print(json.dumps({"mode": mode, "ids": ids, "model": MODELS[mode], "context": render(mode, ids, feats)},
                     ensure_ascii=False))
    return 0


if __name__ == "__main__":
    sys.exit(main())
