#!/usr/bin/env python3
"""Petites opérations de la boucle (hors agent) : contrôles préalables, budget, journal."""
from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from lib import FROZEN_MANIFEST, LOGS, ORACLE, REPO, ensure_state, freeze, load_features, locked, now, save_features  # noqa: E402


def preflight() -> int:
    ensure_state()
    errors = []
    if not load_features()["features"]:
        errors.append("aucune feature : lancer `python3 harness/gen_features.py`")
    if not (ORACLE / "refs" / "base.gcode").exists():
        errors.append("références Orca absentes : lancer `python3 tests/oracle/tools/make_refs.py` (nix develop)")
    if not (ORACLE / "fixtures" / "cube20.stl").exists():
        errors.append("modèles absents : lancer `python3 tests/oracle/fixtures/gen_models.py`")
    dirty = subprocess.run(["git", "status", "--porcelain", "--untracked-files=no"], cwd=REPO,
                           capture_output=True, text=True).stdout.strip()
    if dirty:
        errors.append("arbre git non propre (commit ou stash avant de lancer la boucle) :\n" + dirty[:500])
    for e in errors:
        print("✗ " + e)
    if errors:
        return 1
    if not FROZEN_MANIFEST.exists():
        print(f"gel initial : {freeze()} fichiers")
    return 0


def spent(since: str) -> None:
    runs = LOGS / "runs.jsonl"
    total = 0.0
    if runs.exists():
        for line in runs.read_text().splitlines():
            r = json.loads(line)
            if r.get("at", "") >= since:
                total += r.get("cost_usd") or 0
    print(f"{total:.4f}")


def after(mode: str, ids: list[str], out_file: str, verify_rc: int, session: int, model: str) -> None:
    try:
        res = json.loads(Path(out_file).read_text() or "{}")
    except json.JSONDecodeError:
        res = {"result": Path(out_file).read_text()[-2000:]}
    text = res.get("result") or ""
    m = re.search(r"NOTE:(.*)", text, re.S)
    note = (m.group(1) if m else text[-600:]).strip()[:800]
    usage = res.get("usage") or {}
    tokens = sum(usage.get(k) or 0 for k in ("input_tokens", "output_tokens",
                                              "cache_read_input_tokens", "cache_creation_input_tokens"))
    with locked():
        data = load_features()
        passed = []
        for i in ids:
            f = data["features"][i]
            f["last_note"] = note or None
            if mode == "review":
                f["reviewed"] = True
            for tag in ("TEST_SUSPECT", "TEST_FIX", "SPLIT"):
                t = re.search(tag + r":\s*(.+)", text)
                if t:
                    f["flag"] = f"{tag}: {t.group(1).strip()[:400]}"
                    if tag != "TEST_SUSPECT" and f["status"] != "passing":
                        f["status"], f["reason"] = "needs_review", f["flag"]
            if f["status"] == "passing":
                passed.append(i)
        save_features(data)
    LOGS.mkdir(parents=True, exist_ok=True)
    with open(LOGS / "runs.jsonl", "a") as fh:
        fh.write(json.dumps({"at": now(), "session": session, "mode": mode, "ids": ids, "model": model,
                             "cost_usd": res.get("total_cost_usd"), "tokens": tokens,
                             "turns": res.get("num_turns"), "verify_rc": verify_rc, "passed": passed,
                             "session_id": res.get("session_id")}) + "\n")
    print(f"  → {len(passed)}/{len(ids)} validée(s), ${res.get('total_cost_usd') or 0:.2f}, "
          f"{tokens / 1000:.0f}k tokens, {res.get('num_turns')} tours")


def requires_done(ids: list[str]) -> None:
    """Après une session « tests » sur des paramètres : régénère cas + références Orca des clés concernées."""
    keys = [i[6:] for i in ids if i.startswith("param.") and (ORACLE / "params" / "requires" / f"{i[6:]}.json").exists()]
    if not keys:
        return
    for k in keys:
        for ext in (".gcode", ".error", ".value.json"):
            (ORACLE / "refs" / "param" / f"{k}{ext}").unlink(missing_ok=True)
    with locked():
        data = load_features()
        for k in keys:
            f = data["features"][f"param.{k}"]
            f["status"], f["attempts"] = "needs_ref", 0
            f.pop("reason", None)
        save_features(data)
    subprocess.run([sys.executable, str(REPO / "harness" / "gen_features.py")], cwd=REPO, capture_output=True)
    r = subprocess.run([sys.executable, str(ORACLE / "tools" / "make_refs.py"), "--only", "params", "--keys", ",".join(keys)],
                       cwd=REPO, capture_output=True, text=True)
    print(f"  références Orca régénérées pour {len(keys)} clé(s)" if r.returncode == 0 else
          f"  make_refs.py a échoué (clés restent en needs_ref) : {(r.stdout + r.stderr)[-300:]}")


if __name__ == "__main__":
    cmd = sys.argv[1]
    if cmd == "preflight":
        sys.exit(preflight())
    elif cmd == "freeze":
        print(f"gelé : {freeze()} fichiers")
    elif cmd == "spent":
        spent(sys.argv[2])
    elif cmd == "requires":
        requires_done(sys.argv[2].split())
    elif cmd == "after":
        mode, ids, out_file, rc, session, model = sys.argv[2:8]
        after(mode, ids.split(), out_file, int(rc), int(session), model)
    else:
        sys.exit(f"commande inconnue : {cmd}")
