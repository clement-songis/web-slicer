#!/usr/bin/env python3
"""Référence Orca d'arrangement séquentiel : cube20 + cylinder + bridge,
presets make_refs (Voron 2.4 350), print_sequence = by object / by layer,
`orca-slicer --arrange 1 --orient 0 --export-3mf` (sans tranchage).
5 tirages : la référence est la variante majoritaire (tests/oracle/refs/README.md)."""
import collections, hashlib, json, shutil, subprocess, sys, tempfile, zipfile
from pathlib import Path
sys.path.insert(0, "/var/lib/openclaw/workspace/web-slicer/tests/oracle/tools")
import make_refs

HERE = Path(__file__).resolve().parent
printer, process, filament = make_refs.Refs(make_refs.DEFAULT_PROFILES).trio()
MODELS = ("cube20", "cylinder", "bridge")
for seq in ("by object", "by layer"):
    tag = seq.replace(" ", "-")
    draws = collections.Counter(); keep = {}
    for _ in range(5):
        proc = dict(process); proc["print_sequence"] = seq
        with tempfile.TemporaryDirectory(dir="/var/lib/openclaw/tmp") as tmp:
            t = Path(tmp)
            for n, d in (("machine", printer), ("process", proc), ("filament", filament)):
                (t / f"{n}.json").write_text(json.dumps(d, ensure_ascii=False))
            models = []
            for m in MODELS:
                shutil.copy(make_refs.ORACLE / "fixtures" / f"{m}.stl", t / f"{m}.stl")
                models.append(str(t / f"{m}.stl"))
            out = t / "out"; out.mkdir()
            subprocess.run(make_refs.orca_cmd() + ["--load-settings", f"{t/'machine.json'};{t/'process.json'}",
                            "--load-filaments", str(t / "filament.json"), "--arrange", "1", "--orient", "0",
                            "--export-3mf", "scene.3mf", "--outputdir", str(out)] + models,
                           capture_output=True, text=True, timeout=600)
            z = zipfile.ZipFile(out / "scene.3mf")
            model = z.read("3D/3dmodel.model")
            build = model[model.find(b"<build"):]
            h = hashlib.md5(build).hexdigest()[:8]
            draws[h] += 1
            keep.setdefault(h, (out / "scene.3mf").read_bytes())
    best, n = draws.most_common(1)[0]
    (HERE / f"{tag}.3mf").write_bytes(keep[best])
    print(seq, dict(draws), "->", best, f"{n}/5")
