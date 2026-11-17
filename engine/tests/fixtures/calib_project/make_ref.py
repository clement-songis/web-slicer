#!/usr/bin/env python3
"""Références Orca des projets de calibration (ffi_calib_project) : chaque
<kind>.3mf (écrit par le test ignoré export_calib_projects : modèle, coupes,
échelle, réglages d'objet et presets édités du portage des Plater::calib_*)
est tranché par `orca-slicer --slice 0` ; variante majoritaire sur N tirages.
Le CLI ne connaît pas le mode calibration : références hors injection.
Usage : make_ref.py [N=5]
"""
import collections, hashlib, subprocess, sys, tempfile
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[4] / "tests/oracle/tools"))
import make_refs

HERE = Path(__file__).resolve().parent
N = int(sys.argv[1]) if len(sys.argv) > 1 else 5
ONLY = set(sys.argv[2:])  # noms de projets (sans .3mf) ; vide = tous
for src in sorted(HERE.glob("*.3mf")):
    if ONLY and src.stem not in ONLY:
        continue
    draws = collections.Counter(); keep = {}; err = ""
    for _ in range(N):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            r = subprocess.run(make_refs.orca_cmd() + ["--slice", "0", "--outputdir", str(out), str(src)],
                               capture_output=True, text=True, timeout=900)
            g = sorted(out.glob("*.gcode"))
            if not g:
                draws["ERROR"] += 1; err = (r.stdout + r.stderr)[-600:]; continue
            text = g[0].read_text()
            h = hashlib.md5("\n".join(l for l in text.split("\n") if not l.startswith(";")).encode()).hexdigest()[:8]
            draws[h] += 1; keep.setdefault(h, text)
    best, n = draws.most_common(1)[0]
    dest = src.with_suffix(".orca.gcode")
    if best == "ERROR":
        src.with_suffix(".orca.error").write_text(err); dest.unlink(missing_ok=True)
    elif n * 2 <= N:
        # Pas de variante majoritaire (Orca non déterministe sur ce modèle) :
        # pas de référence (tests/oracle/refs/README.md).
        src.with_suffix(".orca.unstable").write_text(f"{dict(draws)}\n"); dest.unlink(missing_ok=True)
    else:
        dest.write_text(keep[best])
    print(src.stem, dict(draws), "->", best, f"{n}/{N}", flush=True)
