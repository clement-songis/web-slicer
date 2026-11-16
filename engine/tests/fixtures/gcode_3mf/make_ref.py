#!/usr/bin/env python3
"""Fixture « Export all plate sliced file » : projet deux plateaux (cube20 sur
le plateau 1, cylindre sur le plateau 2) dérivé de print_sequence/by-layer.3mf,
puis référence Orca : `orca-slicer --slice 0 --export-3mf` (.gcode.3mf avec
Metadata/plate_<n>.gcode) et G-code par plateau.

Plateau 2 : origine décalée de largeur × (1 + 1/5) en X (PartPlateList,
LOGICAL_PART_PLATE_GAP), soit 420 mm pour le Voron 350.
"""
import re, shutil, subprocess, sys, tempfile, zipfile
from pathlib import Path
sys.path.insert(0, "/var/lib/openclaw/workspace/web-slicer/tests/oracle/tools")
import make_refs

HERE = Path(__file__).resolve().parent
SRC = HERE.parent / "print_sequence" / "by-layer.3mf"
OUT = HERE / "two_plates.3mf"

with zipfile.ZipFile(SRC) as zin, zipfile.ZipFile(OUT, "w", zipfile.ZIP_DEFLATED) as zout:
    for item in zin.infolist():
        data = zin.read(item.filename)
        if item.filename == "Metadata/model_settings.config":
            s = data.decode()
            inst2 = re.search(r"\s*<model_instance>\s*<metadata key=\"object_id\" value=\"4\"/>.*?</model_instance>", s, re.S).group(0)
            s = s.replace(inst2, "")
            plate2 = ("\n  <plate>\n    <metadata key=\"plater_id\" value=\"2\"/>\n    <metadata key=\"plater_name\" value=\"\"/>\n"
                      "    <metadata key=\"locked\" value=\"false\"/>\n    <metadata key=\"filament_map_mode\" value=\"Auto For Flush\"/>\n"
                      "    <metadata key=\"gcode_file\" value=\"\"/>" + inst2 + "\n  </plate>")
            s = s.replace("</plate>", "</plate>" + plate2, 1)
            data = s.encode()
        elif item.filename == "3D/3dmodel.model":
            s = data.decode()
            s = s.replace("175.000001 164 0", "595.000001 164 0")
            data = s.encode()
        zout.writestr(item, data)

with tempfile.TemporaryDirectory() as tmp:
    out = Path(tmp)
    r = subprocess.run(make_refs.orca_cmd() + ["--slice", "0", "--export-3mf", "ref.gcode.3mf",
                        "--outputdir", str(out), str(OUT)], capture_output=True, text=True, timeout=900)
    print((r.stdout + r.stderr)[-400:])
    for f in sorted(out.iterdir()):
        print(" ", f.name, f.stat().st_size)
    shutil.copy(out / "ref.gcode.3mf", HERE / "two_plates.orca.gcode.3mf")
    for n in (1, 2):
        shutil.copy(out / f"plate_{n}.gcode", HERE / f"plate_{n}.orca.gcode")
