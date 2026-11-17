#!/usr/bin/env python3
"""Variantes Orca (non déterminisme) des projets température / rétraction :
N tranchages `orca-slicer --slice 0` de <kind>.3mf, chaque variante distincte
(corps sans commentaires) écrite dans OUT/<kind>/<empreinte>.gcode, compteurs
dans OUT/<kind>/draws.json. Les empreintes du test sont ensuite calculées par
`cargo test --test ffi_calib_project record_orca_variants -- --ignored`
(ORCA_VARIANTS_DIR=OUT).
Usage : collect_variants.py OUT [N=8] [kind…]
"""
import collections, hashlib, json, subprocess, sys, tempfile
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[4] / "tests/oracle/tools"))
import make_refs

HERE = Path(__file__).resolve().parent
out = Path(sys.argv[1])
N = int(sys.argv[2]) if len(sys.argv) > 2 else 8
kinds = sys.argv[3:] or ["temp_tower", "retraction"]
for kind in kinds:
    dest = out / kind
    dest.mkdir(parents=True, exist_ok=True)
    draws = collections.Counter()
    for _ in range(N):
        with tempfile.TemporaryDirectory() as tmp:
            o = Path(tmp)
            subprocess.run(make_refs.orca_cmd() + ["--slice", "0", "--outputdir", str(o), str(HERE / f"{kind}.3mf")],
                           capture_output=True, text=True, timeout=900)
            g = sorted(o.glob("*.gcode"))
            if not g:
                draws["ERROR"] += 1
                continue
            text = g[0].read_text()
            h = hashlib.md5("\n".join(l for l in text.split("\n") if not l.startswith(";")).encode()).hexdigest()[:8]
            draws[h] += 1
            if not (dest / f"{h}.gcode").exists():
                (dest / f"{h}.gcode").write_text(text)
    (dest / "draws.json").write_text(json.dumps(dict(draws), indent=1))
    print(kind, dict(draws), flush=True)
