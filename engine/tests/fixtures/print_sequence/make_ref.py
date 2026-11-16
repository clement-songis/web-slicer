#!/usr/bin/env python3
"""Références Orca d'impression séquentielle (print_sequence = by object) :
deux objets distincts (cube20 + cylinder) arrangés par le CLI, presets make_refs."""
import json, shutil, subprocess, sys, tempfile
from pathlib import Path
sys.path.insert(0, "/var/lib/openclaw/workspace/web-slicer/tests/oracle/tools")
import make_refs

HERE = Path(__file__).resolve().parent
printer, process, filament = make_refs.Refs(make_refs.DEFAULT_PROFILES).trio()
for seq in ("by object", "by layer"):
    proc = dict(process)
    proc["print_sequence"] = seq
    with tempfile.TemporaryDirectory() as tmp:
        t = Path(tmp)
        for n, d in (("machine", printer), ("process", proc), ("filament", filament)):
            (t / f"{n}.json").write_text(json.dumps(d, ensure_ascii=False))
        models = []
        for m in ("cube20", "cylinder"):
            shutil.copy(make_refs.ORACLE / "fixtures" / f"{m}.stl", t / f"{m}.stl")
            models.append(str(t / f"{m}.stl"))
        out = t / "out"; out.mkdir()
        r = subprocess.run(make_refs.orca_cmd() + ["--load-settings", f"{t/'machine.json'};{t/'process.json'}",
                            "--load-filaments", str(t / "filament.json"), "--arrange", "1", "--orient", "0",
                            "--export-3mf", "scene.3mf", "--outputdir", str(out)] + models,
                           capture_output=True, text=True, timeout=600)
        tag = seq.replace(" ", "-")
        if not (out / "scene.3mf").exists():
            print(seq, "ÉCHEC export", (r.stdout + r.stderr)[-300:]); continue
        shutil.copy(out / "scene.3mf", HERE / f"{tag}.3mf")
        # Référence = tranchage par Orca du 3MF exporté (même chemin de lecture
        # que le moteur : options créées par le CLI à la lecture d'un 3MF).
        o2 = t / "o2"; o2.mkdir()
        subprocess.run(make_refs.orca_cmd() + ["--slice", "0", "--outputdir", str(o2), str(HERE / f"{tag}.3mf")],
                       capture_output=True, text=True, timeout=600)
        g = sorted(o2.glob("*.gcode"))
        if g:
            shutil.copy(g[0], HERE / f"{tag}.orca.gcode")
        print(seq, "gcode" if g else "ÉCHEC tranchage 3MF")
