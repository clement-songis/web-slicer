#!/usr/bin/env python3
"""Référence Orca « changement de couleur + pause par couche » :
1. orca-slicer exporte un 3MF du cube20 (presets de make_refs, arrangé) ;
2. on y ajoute Metadata/custom_gcode_per_layer.xml (format bbs_3mf.cpp:8462) ;
3. orca-slicer tranche ce 3MF → G-code de référence."""
import json, shutil, subprocess, sys, tempfile, zipfile
from pathlib import Path
sys.path.insert(0, "/var/lib/openclaw/workspace/web-slicer/tests/oracle/tools")
import make_refs

HERE = Path(__file__).resolve().parent
printer, process, filament = make_refs.Refs(make_refs.DEFAULT_PROFILES).trio()
with tempfile.TemporaryDirectory() as tmp:
    t = Path(tmp)
    for n, d in (("machine", printer), ("process", process), ("filament", filament)):
        (t / f"{n}.json").write_text(json.dumps(d, ensure_ascii=False))
    stl = t / "cube20.stl"
    shutil.copy(make_refs.ORACLE / "fixtures" / "cube20.stl", stl)
    base = make_refs.orca_cmd()
    out = t / "o1"; out.mkdir()
    subprocess.run(base + ["--load-settings", f"{t/'machine.json'};{t/'process.json'}",
                           "--load-filaments", str(t / "filament.json"), "--arrange", "1", "--orient", "0",
                           "--export-3mf", "cube.3mf", "--outputdir", str(out), str(stl)],
                   check=False, capture_output=True, text=True, timeout=600)
    src = out / "cube.3mf"
    if not src.exists():
        sys.exit("export 3MF Orca échoué")
    shutil.copy(src, HERE / "cube.3mf")
    xml = """<?xml version="1.0" encoding="utf-8"?>
<custom_gcodes_per_layer>
<plate>
<plate_info id="1"/>
<layer top_z="5" type="0" extruder="1" color="#FF0000" extra="" gcode="M600"/>
<layer top_z="10" type="1" extruder="1" color="" extra="Pause test" gcode="M601"/>
<layer top_z="15" type="4" extruder="1" color="" extra="M117 Couche 15" gcode="M117 Couche 15"/>
<mode value="SingleExtruder"/>
</plate>
</custom_gcodes_per_layer>
"""
    dst = HERE / "cube_custom.3mf"
    with zipfile.ZipFile(src) as zin, zipfile.ZipFile(dst, "w", zipfile.ZIP_DEFLATED) as zout:
        for item in zin.infolist():
            if item.filename != "Metadata/custom_gcode_per_layer.xml":
                zout.writestr(item, zin.read(item.filename))
        zout.writestr("Metadata/custom_gcode_per_layer.xml", xml)
    for name in ("cube.3mf", "cube_custom.3mf"):
        o = t / ("s-" + name); o.mkdir()
        r = subprocess.run(base + ["--slice", "0", "--outputdir", str(o), str(HERE / name)],
                           capture_output=True, text=True, timeout=600)
        g = sorted(o.glob("*.gcode"))
        if not g:
            print(name, "échec", (r.stdout + r.stderr)[-800:]); continue
        shutil.copy(g[0], HERE / (name.replace(".3mf", ".orca.gcode")))
        print(name, "ok", g[0].stat().st_size)
