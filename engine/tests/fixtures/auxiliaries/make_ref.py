#!/usr/bin/env python3
"""Fichiers auxiliaires : project_aux.3mf (écrit par le moteur, test ignoré
export_aux_project : Auxiliaries/Model Pictures/vue.png, Others/notes.txt,
Bill of Materials/bom.csv) rouvert puis réexporté par le CLI Orca
(--export-3mf) → project_aux.orca.3mf : Orca conserve-t-il les mêmes entrées ?
"""
import shutil, subprocess, sys, tempfile
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[4] / "tests/oracle/tools"))
import make_refs

HERE = Path(__file__).resolve().parent
with tempfile.TemporaryDirectory() as tmp:
    t = Path(tmp)
    r = subprocess.run(make_refs.orca_cmd() + ["--export-3mf", "out.3mf", "--outputdir", str(t), str(HERE / "project_aux.3mf")],
                       capture_output=True, text=True, timeout=600)
    if not (t / "out.3mf").exists():
        raise SystemExit((r.stdout + r.stderr)[-1500:])
    shutil.copy(t / "out.3mf", HERE / "project_aux.orca.3mf")
    print("ok")
