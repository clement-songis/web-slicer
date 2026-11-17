#!/usr/bin/env python3
"""Scène 2 objets / 2 filaments sur une autre imprimante du catalogue (bloquants
phase 15) : export par le CLI Orca avec les presets donnés (deux filaments,
tour d'essuyage), extrudeur de l'objet 2 posé à 2, puis référence
`orca-slicer --slice 0` (variante majoritaire sur N tirages) ou .error.

Usage : make_printer_scene.py NOM "machine" "process" "filament" [N]
"""
import collections, hashlib, json, re, shutil, subprocess, sys, tempfile, zipfile
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[4] / "tests/oracle/tools"))
import make_refs

HERE = Path(__file__).resolve().parent
name, machine, process_name, filament_name = sys.argv[1:5]
N = int(sys.argv[5]) if len(sys.argv) > 5 and sys.argv[5].isdigit() else 5
# --project '{"clé": valeur}' : réglages du projet posés dans project_settings.config
# (ex. H2D : filament_map_mode = Manual, filament_map = [1, 2] → un filament par buse).
PROJECT = json.loads(sys.argv[sys.argv.index("--project") + 1]) if "--project" in sys.argv else {}
refs = make_refs.Refs(make_refs.DEFAULT_PROFILES)
printer, process, filament = refs.preset(machine), refs.preset(process_name), refs.preset(filament_name)
proc = dict(process); proc["enable_prime_tower"] = "1"
f2 = dict(filament)
# Le CLI refuse (CLI_CONFIG_FILE_ERROR, -5) une couleur explicite quand le preset
# filament n'a pas filament_is_support (vecteur vide ≠ nombre de filaments,
# OrcaSlicer.cpp ~3400) : cas Prusa XL. --same-colour garde la couleur du preset.
if "--same-colour" not in sys.argv:
    f2["filament_colour"] = ["#FF0000"]


def run(cmd, cwd=None):
    return subprocess.run(make_refs.orca_cmd() + cmd, capture_output=True, text=True, timeout=900, cwd=cwd)


with tempfile.TemporaryDirectory() as tmp:
    t = Path(tmp)
    for n, d in (("machine", printer), ("process", proc), ("f1", filament), ("f2", f2)):
        (t / f"{n}.json").write_text(json.dumps(d, ensure_ascii=False))
    for m in ("cube20", "cylinder"):
        shutil.copy(make_refs.ORACLE / "fixtures" / f"{m}.stl", t / f"{m}.stl")
    r = run(["--load-settings", f"{t/'machine.json'};{t/'process.json'}",
             "--load-filaments", f"{t/'f1.json'};{t/'f2.json'}", "--arrange", "1", "--orient", "0",
             "--export-3mf", "scene.3mf", "--outputdir", str(t), str(t / "cube20.stl"), str(t / "cylinder.stl")])
    if not (t / "scene.3mf").exists():
        raise SystemExit(f"export refusé : {(r.stdout + r.stderr)[-800:]}")
    src = zipfile.ZipFile(t / "scene.3mf")
    dest = HERE / f"{name}.3mf"
    with zipfile.ZipFile(dest, "w", zipfile.ZIP_DEFLATED) as z:
        for i in src.infolist():
            d = src.read(i.filename)
            if i.filename == "Metadata/model_settings.config":
                s = d.decode()
                s = re.sub(r'(<metadata key="name" value="cylinder.stl"/>\s*<metadata key="extruder" value=")1', r"\g<1>2", s, count=1)
                d = s.encode()
            elif i.filename == "Metadata/project_settings.config" and PROJECT:
                cfg = json.loads(d)
                cfg.update(PROJECT)
                d = json.dumps(cfg, indent=4, ensure_ascii=False).encode()
            z.writestr(i, d)

draws = collections.Counter(); keep = {}; why = ""
for _ in range(N):
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp)
        r = run(["--slice", "0", "--outputdir", str(out), str(dest)])
        g = sorted(out.glob("*.gcode"))
        if not g:
            res = out / "result.json"
            why = (json.loads(res.read_text()).get("error_string", "") if res.exists() else "") + "\n" + (r.stdout + r.stderr)[-1500:]
            draws["ERROR"] += 1
            continue
        text = g[0].read_text()
        h = hashlib.md5("\n".join(l for l in text.split("\n") if not l.startswith(";")).encode()).hexdigest()[:8]
        draws[h] += 1; keep.setdefault(h, text)
best, n = draws.most_common(1)[0]
if best == "ERROR":
    (HERE / f"{name}.orca.error").write_text(why)
else:
    (HERE / f"{name}.orca.gcode").write_text(keep[best])
print(name, dict(draws), "->", best, f"{n}/{N}")
