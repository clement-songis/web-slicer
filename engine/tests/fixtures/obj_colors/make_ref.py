#!/usr/bin/env python3
"""Références Orca de l'import OBJ coloré : le CLI importe les couleurs d'un OBJ
(sommets ou MTL) seulement via --load-assemble-list (OrcaSlicer.cpp
construct_assemble_list → convert_obj_cluster_colors, obj_import_*_color_deal,
filament_colour = couleurs regroupées). Chaque <nom>.obj est exporté en
<nom>.orca.3mf (peinture MMU par facette + filament_colour).
"""
import json, shutil, subprocess, sys, tempfile
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[4] / "tests/oracle/tools"))
import make_refs

HERE = Path(__file__).resolve().parent
printer, process, filament = make_refs.Refs(make_refs.DEFAULT_PROFILES).trio()
for obj in sorted(HERE.glob("*.obj")):
    with tempfile.TemporaryDirectory() as tmp:
        t = Path(tmp)
        for f in HERE.glob(obj.stem + ".*"):
            shutil.copy(f, t / f.name)
        for n, d in (("machine", printer), ("process", process), ("filament", filament)):
            (t / f"{n}.json").write_text(json.dumps(d))
        assemble = {"plates": [{"plate_name": "", "need_arrange": False,
                                "objects": [{"path": str(t / obj.name), "count": 1, "filaments": [1],
                                             "pos_x": [100.0], "pos_y": [100.0], "pos_z": [0.0]}]}]}
        (t / "assemble.json").write_text(json.dumps(assemble))
        out = t / "out"; out.mkdir()
        r = subprocess.run(make_refs.orca_cmd() + ["--load-settings", f"{t/'machine.json'};{t/'process.json'}",
                           "--load-filaments", str(t / "filament.json"), "--load-assemble-list", str(t / "assemble.json"),
                           "--export-3mf", "out.3mf", "--outputdir", str(out)], capture_output=True, text=True, timeout=600)
        if not (out / "out.3mf").exists():
            raise SystemExit(f"{obj.name} : CLI {r.returncode}\n{(r.stdout + r.stderr)[-1500:]}")
        shutil.copy(out / "out.3mf", HERE / f"{obj.stem}.orca.3mf")
        print(obj.name, "ok", flush=True)
