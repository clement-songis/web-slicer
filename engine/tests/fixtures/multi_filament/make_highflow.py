#!/usr/bin/env python3
"""Débit de buse Standard / High Flow (lot 39) : copie SRC.3mf en DEST.3mf avec
`nozzle_volume_type` = VALEURS (une par extrudeur, ex. "Standard,High Flow")
dans project_settings.config, puis référence `orca-slicer --slice 0`
(variante majoritaire sur N tirages) → DEST.orca.gcode.

Usage : make_highflow.py SRC DEST "High Flow,Standard" [N] [--set '{"clé": valeur}']
(--set : autres réglages du projet, ex. vecteurs par variante à valeurs
distinctes pour rendre visible la variante choisie.)
"""
import collections, hashlib, json, subprocess, sys, tempfile, zipfile
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[4] / "tests/oracle/tools"))
import make_refs

HERE = Path(__file__).resolve().parent
src_name, dest_name, values = sys.argv[1:4]
N = int(sys.argv[4]) if len(sys.argv) > 4 and sys.argv[4].isdigit() else 3
EXTRA = json.loads(sys.argv[sys.argv.index("--set") + 1]) if "--set" in sys.argv else {}
VOLUMES = values.split(",")
src = zipfile.ZipFile(HERE / f"{src_name}.3mf")
dest = HERE / f"{dest_name}.3mf"
with zipfile.ZipFile(dest, "w", zipfile.ZIP_DEFLATED) as z:
    for i in src.infolist():
        d = src.read(i.filename)
        if i.filename == "Metadata/project_settings.config":
            cfg = json.loads(d)
            cfg["nozzle_volume_type"] = VOLUMES
            cfg.update(EXTRA)
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
(HERE / f"{dest_name}.orca.gcode").write_text(keep[best])
print(dest_name, dict(draws), "->", best, f"{k}/{N}")
