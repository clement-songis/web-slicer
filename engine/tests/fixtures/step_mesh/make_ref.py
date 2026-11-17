#!/usr/bin/env python3
"""Référence Orca du maillage STEP : le CLI importe un .step avec les réglages
par défaut (Step::mesh : 0,003 mm / 0,5 rad / sans découpe) et l'exporte en
3MF → <nom>.orca.3mf (comparé au maillage du moteur aux mêmes réglages)."""
import shutil, subprocess, sys, tempfile
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[4] / "tests/oracle/tools"))
import make_refs

HERE = Path(__file__).resolve().parent
for step in sorted(HERE.glob("*.step")):
    with tempfile.TemporaryDirectory() as tmp:
        t = Path(tmp)
        r = subprocess.run(make_refs.orca_cmd() + ["--orient", "0", "--arrange", "0", "--export-3mf", "out.3mf",
                           "--outputdir", str(t), str(step)], capture_output=True, text=True, timeout=600)
        if not (t / "out.3mf").exists():
            print(step.name, "CLI refuse", r.returncode, (r.stdout + r.stderr)[-400:].replace("\n", " | "))
            continue
        shutil.copy(t / "out.3mf", HERE / f"{step.stem}.orca.3mf")
        print(step.name, "ok")
