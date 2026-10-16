#!/usr/bin/env python3
"""Juge unique : exécute les tests oracle d'un lot et met à jour l'état.

  verify.py ID... [--mode impl|tests|review]   # par la boucle : écrit l'état
  verify.py --check ID...                       # par l'agent : lecture seule
  verify.py --all                               # régression complète (nuit)

Codes de sortie : 0 tout vert · 1 échec · 2 régression · 4 altération des tests.
Seul ce script (lancé par la boucle) peut passer une feature à `passing`.
"""
from __future__ import annotations

import argparse
import json
import os
import random
import re
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from lib import (HARNESS, MAX_ATTEMPTS, ORACLE, REPO, STATE, load_features, locked, now,  # noqa: E402
                 restore_frozen, save_features, tamper_report)

ANSI = re.compile(r"\x1b\[[0-9;]*m")
TAG = re.compile(r"@F:([A-Za-z0-9_.\-]+)")
REGRESSION_SAMPLE = int(os.environ.get("HARNESS_REGRESSION_SAMPLE", "8"))


def sh(cmd: list[str], timeout: int, **kw) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, cwd=REPO, capture_output=True, text=True, timeout=timeout, **kw)


def stack_up() -> str | None:
    """Démarre/rafraîchit la pile de test. Renvoie un message d'erreur ou None."""
    if os.environ.get("HARNESS_NO_STACK"):
        return None
    try:
        r = sh([str(HARNESS / "stack.sh"), "up"], timeout=int(os.environ.get("HARNESS_STACK_TIMEOUT", "1800")))
    except subprocess.TimeoutExpired:
        return "stack.sh up : délai dépassé"
    if r.returncode != 0:
        return "stack.sh up a échoué :\n" + (r.stdout + r.stderr)[-2500:]
    return None


def stack_env() -> dict[str, str]:
    """Variables publiées par stack.sh (BASE_URL, MOONRAKER_URL…)."""
    p = STATE / "stack.env"
    if not p.exists():
        return {}
    pairs = (line.split("=", 1) for line in p.read_text().splitlines() if "=" in line)
    return {k.strip(): v.strip() for k, v in pairs}


def run_playwright(ids: list[str] | None) -> dict[str, dict]:
    """Lance Playwright filtré sur les tags et renvoie {id: {outcome, error}}."""
    out = Path(tempfile.mkstemp(suffix=".json")[1])
    cmd = ["npx", "playwright", "test", "-c", str(ORACLE / "playwright.config.ts"), "--reporter=json"]
    if ids:
        alt = "|".join(re.escape(i) for i in ids)
        cmd += ["--grep", f"@F:({alt})(?![A-Za-z0-9_.\\-])"]
    env = {**os.environ, **stack_env(), "PLAYWRIGHT_JSON_OUTPUT_NAME": str(out)}
    try:
        r = subprocess.run(cmd, cwd=ORACLE, capture_output=True, text=True, env=env,
                           timeout=int(os.environ.get("HARNESS_TEST_TIMEOUT", "5400")))
    except subprocess.TimeoutExpired:
        return {i: {"outcome": "failed", "error": "délai global des tests dépassé"} for i in ids or []}
    try:
        report = json.loads(out.read_text())
    except Exception:
        msg = ANSI.sub("", (r.stdout + r.stderr))[-2000:]
        return {i: {"outcome": "error", "error": "Playwright n'a pas produit de rapport :\n" + msg} for i in ids or []}
    finally:
        out.unlink(missing_ok=True)

    res: dict[str, dict] = {}

    def walk(suite: dict, titles: list[str]) -> None:
        for s in suite.get("suites", []):
            walk(s, titles + [s.get("title", "")])
        for spec in suite.get("specs", []):
            full = " ".join(titles + [spec.get("title", "")] + [f"@{t.lstrip('@')}" for t in spec.get("tags", [])])
            tags = set(TAG.findall(full))
            for test in spec.get("tests", []):
                results = test.get("results") or [{}]
                last = results[-1]
                status = last.get("status") or test.get("status") or "skipped"
                err = ANSI.sub("", (last.get("error") or {}).get("message", "") or "")
                loc = (last.get("error") or {}).get("location") or {}
                if loc:
                    err += f"\n  à {Path(loc.get('file', '')).name}:{loc.get('line')}"
                skip_reason = next((a.get("description", "") for a in test.get("annotations", [])
                                    if a.get("type") == "skip"), "")
                for t in tags:
                    cur = res.setdefault(t, {"outcome": "passed", "error": "", "count": 0})
                    cur["count"] += 1
                    if status in ("failed", "timedOut", "interrupted"):
                        cur["outcome"] = "failed"
                        cur["error"] += f"[{spec.get('title')}] {err.strip()[:1200]}\n"
                    elif status == "skipped" and cur["outcome"] == "passed":
                        cur["outcome"] = "skipped"
                        cur["error"] += f"[{spec.get('title')}] ignoré : {skip_reason}\n"
    for s in report.get("suites", []):
        walk(s, [s.get("title", "")])
    for e in report.get("errors", []):
        for i in ids or []:
            res.setdefault(i, {"outcome": "error", "error": "", "count": 0})
            res[i]["outcome"] = "error"  # erreur de chargement (syntaxe, import) : pas un « rouge » valable
            res[i]["error"] += ANSI.sub("", e.get("message", ""))[:800] + "\n"
    for i in ids or []:
        res.setdefault(i, {"outcome": "no_test", "error": "aucun test portant ce tag n'a été trouvé", "count": 0})
    return res


def head_commit() -> str:
    try:
        return sh(["git", "rev-parse", "--short", "HEAD"], 10).stdout.strip()
    except Exception:
        return "?"


def record(f: dict, mode: str, outcome: str, err: str, commit: str) -> None:
    f.setdefault("history", []).append({"at": now(), "mode": mode, "outcome": outcome, "commit": commit})
    f["history"] = f["history"][-12:]
    f["last_error"] = err.strip()[:3000] or None


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("ids", nargs="*")
    ap.add_argument("--check", action="store_true", help="lecture seule (usage agent)")
    ap.add_argument("--mode", default="impl", choices=["impl", "tests", "review"])
    ap.add_argument("--all", action="store_true")
    ap.add_argument("--no-regression", action="store_true")
    args = ap.parse_args()

    # 1. Altération des tests gelés.
    tamper = tamper_report()
    if args.mode == "tests":
        allowed = {f"ajouté: tests/oracle/scenarios/{i}.spec.ts" for i in args.ids}
        allowed |= {f"modifié: tests/oracle/scenarios/{i}.spec.ts" for i in args.ids}
        tamper = [t for t in tamper if t not in allowed]
    if tamper:
        print("ALTÉRATION des chemins gelés :\n  " + "\n  ".join(tamper[:20]))
        if args.check:
            return 4
        restore_frozen()
        with locked():
            data = load_features()
            for i in args.ids:
                f = data["features"][i]
                f["attempts"] = f.get("attempts", 0) + 1
                record(f, args.mode, "tampered", "Tests gelés modifiés (restaurés) : " + "; ".join(tamper[:5]), head_commit())
            save_features(data)
        return 4

    # 2. Pile de test.
    err = stack_up()
    if err:
        print(err)
        if not args.check:
            with locked():
                data = load_features()
                for i in args.ids:
                    f = data["features"][i]
                    f["attempts"] = f.get("attempts", 0) + 1
                    record(f, args.mode, "stack_failed", err, head_commit())
                    if f["attempts"] >= MAX_ATTEMPTS:
                        f["status"] = "blocked"
                save_features(data)
        return 1

    # 3. Tests du lot.
    if args.all:
        return run_all()
    res = run_playwright(args.ids)
    ok = [i for i in args.ids if res[i]["outcome"] == "passed"]
    bad = [i for i in args.ids if i not in ok]
    for i in args.ids:
        print(f"{res[i]['outcome']:8} {i}" + (f"\n{res[i]['error'][:1500]}" if i in bad else ""))
    if args.check:
        return 0 if not bad else 1

    # 4. Régression sur un échantillon de features déjà validées.
    regressions: list[str] = []
    if args.mode == "impl" and not args.no_regression:
        data = load_features()
        pool = [k for k, f in data["features"].items() if f["status"] == "passing" and k not in args.ids]
        sample = random.sample(pool, min(REGRESSION_SAMPLE, len(pool)))
        if sample:
            rr = run_playwright(sample)
            regressions = [k for k in sample if rr[k]["outcome"] in ("failed", "error")]
            if regressions:
                print("RÉGRESSION : " + ", ".join(regressions))
                reg_msg = "\n".join(f"{k}: {rr[k]['error'][:400]}" for k in regressions)

    # 5. Mise à jour de l'état.
    commit = head_commit()
    with locked():
        data = load_features()
        for i in args.ids:
            f = data["features"][i]
            o, e = res[i]["outcome"], res[i]["error"]
            if regressions:
                f["attempts"] = f.get("attempts", 0) + 1
                record(f, args.mode, "regression", "Ton changement casse des features validées :\n" + reg_msg, commit)
            elif args.mode == "tests":
                if o == "failed":
                    f["status"] = "red"
                    record(f, "tests", "red", "", commit)
                elif o == "passed":
                    f["status"], f["green_on_creation"] = "passing", True
                    record(f, "tests", "passed", "", commit)
                else:
                    f["attempts"] = f.get("attempts", 0) + 1
                    record(f, "tests", o, e or "test absent", commit)
            elif o == "passed":
                f["status"], f["attempts"] = "passing", 0
                record(f, args.mode, "passed", "", commit)
            elif o == "skipped":
                f["status"] = "needs_review"
                f["reason"] = e[:400]
                record(f, args.mode, "skipped", e, commit)
            else:
                f["attempts"] = f.get("attempts", 0) + 1
                record(f, args.mode, o, e, commit)
            if f["status"] not in ("passing", "needs_review") and f.get("attempts", 0) >= MAX_ATTEMPTS:
                f["status"] = "blocked"
        save_features(data)
    if regressions:
        return 2
    return 0 if not bad else 1


def run_all() -> int:
    data = load_features()
    ids = [k for k, f in data["features"].items() if f["status"] in ("passing", "red")]
    res = run_playwright(ids)
    flipped = []
    with locked():
        data = load_features()
        for k in ids:
            f = data["features"][k]
            o = res[k]["outcome"]
            if f["status"] == "passing" and o in ("failed", "error"):
                f["status"] = "red"
                record(f, "nightly", "regressed", res[k]["error"], head_commit())
                flipped.append(k)
            elif f["status"] == "red" and o == "passed":
                f["status"] = "passing"
                record(f, "nightly", "passed", "", head_commit())
        save_features(data)
    print(f"{len(ids)} testées, {len(flipped)} régressions : {', '.join(flipped[:30])}")
    return 2 if flipped else 0


if __name__ == "__main__":
    sys.exit(main())
