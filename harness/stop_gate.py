#!/usr/bin/env python3
"""Hook Stop : avant de rendre la main, l'agent d'implémentation doit avoir ses tests verts.

Une seule relance par session (stop_hook_active) : si c'est encore rouge ensuite,
la session se termine et la boucle compte la tentative.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent


def main() -> None:
    data = json.load(sys.stdin)
    mode = os.environ.get("HARNESS_MODE")
    ids = os.environ.get("HARNESS_FEATURE", "").split()
    if mode not in ("impl", "review") or not ids or data.get("stop_hook_active"):
        return
    try:
        r = subprocess.run([sys.executable, str(REPO / "harness" / "verify.py"), "--check", *ids],
                           cwd=REPO, capture_output=True, text=True, timeout=1700)
    except subprocess.TimeoutExpired:
        return
    if r.returncode == 0:
        return
    out = (r.stdout + r.stderr)[-1800:]
    print(json.dumps({
        "decision": "block",
        "reason": "Les tests du lot ne passent pas encore :\n" + out
                  + "\nCorrige si la cause est claire. Sinon, commit ce qui avance et termine par "
                    "« NOTE: » avec la cause précise et la piste suivante.",
    }, ensure_ascii=False))


if __name__ == "__main__":
    main()
