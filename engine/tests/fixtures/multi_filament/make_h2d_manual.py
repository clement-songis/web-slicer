#!/usr/bin/env python3
"""Bambu H2D en affectation MANUELLE filament → buse : bambu_h2d.3mf (auto :
filament_map = 2,1) avec filament_map_mode = Manual et filament_map = 1,2 dans
la config du projet ET dans le plateau (model_settings.config : le CLI lit le
mode du plateau, PartPlate::get_real_filament_map_mode) → bambu_h2d_manual.3mf ;
référence `orca-slicer --slice 0`, variante majoritaire sur N tirages.
"""
import collections, hashlib, json, re, subprocess, sys, tempfile, zipfile
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[4] / "tests/oracle/tools"))
import make_refs

HERE = Path(__file__).resolve().parent
N = int(sys.argv[1]) if len(sys.argv) > 1 else 5
MAP = ["1", "2"]
src = zipfile.ZipFile(HERE / "bambu_h2d.3mf")
dest = HERE / "bambu_h2d_manual.3mf"
with zipfile.ZipFile(dest, "w", zipfile.ZIP_DEFLATED) as z:
    for i in src.infolist():
        d = src.read(i.filename)
        if i.filename == "Metadata/project_settings.config":
            cfg = json.loads(d)
            cfg["filament_map_mode"] = "Manual"
            cfg["filament_map"] = MAP
            d = json.dumps(cfg, indent=4, ensure_ascii=False).encode()
        elif i.filename == "Metadata/model_settings.config":
            s = d.decode()
            s = re.sub(r'\s*<metadata key="filament_map(s|_mode)" value="[^"]*"/>', "", s)
            s = s.replace("<plate>", '<plate>\n    <metadata key="filament_map_mode" value="Manual"/>\n'
                          f'    <metadata key="filament_maps" value="{" ".join(MAP)}"/>', 1)
            d = s.encode()
        z.writestr(i, d)
draws = collections.Counter(); keep = {}
for _ in range(N):
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp)
        subprocess.run(make_refs.orca_cmd() + ["--slice", "0", "--outputdir", str(out), str(dest)],
                       capture_output=True, text=True, timeout=900)
        g = sorted(out.glob("*.gcode"))
        if not g:
            draws["ERROR"] += 1; continue
        text = g[0].read_text()
        h = hashlib.md5("\n".join(l for l in text.split("\n") if not l.startswith(";")).encode()).hexdigest()[:8]
        draws[h] += 1; keep.setdefault(h, text)
best, k = draws.most_common(1)[0]
if best == "ERROR" or k * 2 <= N:
    raise SystemExit(f"pas de variante majoritaire {dict(draws)}")
text = keep[best]
(HERE / "bambu_h2d_manual.orca.gcode").write_text(text)
fm = re.search(r"^; filament_map = (.*)$", text, re.M)
print("bambu_h2d_manual", dict(draws), "->", best, f"{k}/{N}", "filament_map =", fm.group(1) if fm else "?")
