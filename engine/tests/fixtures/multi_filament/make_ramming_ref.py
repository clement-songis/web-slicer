#!/usr/bin/env python3
"""Référence Orca du ramming en SEMM (scène SeeMeCNC Artemis, tour type2) :
enable_filament_ramming = 1 et une courbe filament_ramming_parameters
personnalisée (format du dialogue Ramming d'Orca : « vitesse_ligne
largeur_ligne v1 … v20| t1 v1 t2 v2 … »), posés dans project_settings.config
de seemecnc_artemis.3mf → seemecnc_ramming.3mf ; `orca-slicer --slice 0`,
variante majoritaire sur N tirages → seemecnc_ramming.orca.gcode.
"""
import collections, hashlib, json, subprocess, sys, tempfile, zipfile
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[4] / "tests/oracle/tools"))
import make_refs

HERE = Path(__file__).resolve().parent
N = int(sys.argv[1]) if len(sys.argv) > 1 else 5
RAMMING = "150 80 3.1 4.2 5.5 6.8 7.6 8.1 8.8 9.4 9.9 10.4| 0.05 3.1 0.55 4.2 1.05 5.5 1.55 6.8 2.05 7.6 2.55 8.1 3.05 8.8 3.55 9.4 4.05 9.9"
src = zipfile.ZipFile(HERE / "seemecnc_artemis.3mf")
cfg = json.loads(src.read("Metadata/project_settings.config"))
n = len(cfg.get("filament_ramming_parameters", [None, None])) or 2
cfg["enable_filament_ramming"] = "1"
cfg["filament_ramming_parameters"] = [RAMMING] * n
dest = HERE / "seemecnc_ramming.3mf"
with zipfile.ZipFile(dest, "w", zipfile.ZIP_DEFLATED) as z:
    for i in src.infolist():
        d = src.read(i.filename)
        if i.filename == "Metadata/project_settings.config":
            d = json.dumps(cfg, indent=4, ensure_ascii=False).encode()
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
(HERE / "seemecnc_ramming.orca.gcode").write_text(keep[best])
print("seemecnc_ramming", dict(draws), "->", best, f"{k}/{N}")
