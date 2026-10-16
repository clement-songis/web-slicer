#!/usr/bin/env python3
"""Tableau de bord : avancement par catégorie, coût, files d'attente humaines."""
from __future__ import annotations

import json
import sys
from collections import Counter, defaultdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from lib import LOGS, load_features  # noqa: E402

feats = load_features()["features"]
cols = ["passing", "red", "needs_test", "needs_ref", "blocked", "needs_review", "excluded"]
by = defaultdict(Counter)
for f in feats.values():
    by[f["kind"]][f["status"]] += 1

print(f"{'catégorie':10}" + "".join(f"{c:>13}" for c in cols) + f"{'%':>7}")
tot = Counter()
for kind in sorted(by, key=lambda k: min(f.get("priority", 99) for f in feats.values() if f["kind"] == k)):
    c = by[kind]
    tot += c
    todo = sum(c.values()) - c["excluded"]
    pct = 100 * c["passing"] / todo if todo else 100
    print(f"{kind:10}" + "".join(f"{c[s]:>13}" for s in cols) + f"{pct:>6.0f}%")
todo = sum(tot.values()) - tot["excluded"]
print(f"{'TOTAL':10}" + "".join(f"{tot[s]:>13}" for s in cols) + f"{100 * tot['passing'] / max(todo, 1):>6.0f}%")

runs = LOGS / "runs.jsonl"
if runs.exists():
    rows = [json.loads(line) for line in runs.read_text().splitlines() if line.strip()]
    cost = sum(r.get("cost_usd") or 0 for r in rows)
    toks = sum(r.get("tokens") or 0 for r in rows)
    won = sum(len(r.get("passed", [])) for r in rows)
    by_mode = Counter()
    for r in rows:
        by_mode[r.get("mode")] += r.get("cost_usd") or 0
    print(f"\n{len(rows)} sessions · {toks/1e6:.1f} M tokens · ${cost:.2f} · {won} features validées"
          f" · ${cost / max(won, 1):.2f}/feature")
    print("  par mode : " + ", ".join(f"{m} ${c:.2f}" for m, c in by_mode.items()))


def show(title, items):
    if items:
        print(f"\n{title} ({len(items)})")
        for k, why in items[:25]:
            print(f"  {k:45} {str(why or '')[:110]}")


show("BLOQUÉES — à regarder", [(k, (f.get("last_error") or "").splitlines()[0] if f.get("last_error") else "")
                              for k, f in feats.items() if f["status"] == "blocked"])
show("À REVOIR (humain)", [(k, f.get("reason")) for k, f in feats.items() if f["status"] == "needs_review"])
show("Tests verts dès leur création (vérifier qu'ils testent vraiment quelque chose)",
     [(k, f["test"]) for k, f in feats.items() if f.get("green_on_creation") and not f.get("green_checked")])
