#!/usr/bin/env python3
"""Lot 47 (audit 45 EARS-a) : oreilles de bordure peintes (outil E d'Orca).

print_sequence/by-layer.3mf (cube20 + cylindre, Voron 2.4 350) avec
brim_type = painted et quatre oreilles aux coins bas du cube, écrites au
format d'Orca (bbs_3mf.cpp, _add_brim_ear_points_file_to_archive :
Metadata/brim_ear_points.txt, « brim_points_format_version=0 » puis
« object_id=<rang 1-based>|x y z r ... », repère objet, z monde = -0.0001
comme GLGizmoBrimEars). Référence : orca-slicer --slice 0 du 3MF.
Le 3MF de base est d'abord réexporté par le CLI (--export-3mf) : Orca
recentre chaque objet sur son origine au chargement (composant à 0, instance
relevée) SANS déplacer brim_points ; les oreilles sont donc exprimées dans ce
repère recentré, celui d'un projet enregistré par Orca.
Régénérer : python3 make_ref.py (nix develop).
"""
import re, shutil, subprocess, sys, tempfile, zipfile, json
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[4] / "tests/oracle/tools"))
import make_refs

HERE = Path(__file__).resolve().parent
SRC = HERE.parent / "print_sequence" / "by-layer.3mf"
DEST = HERE / "brim-ears.3mf"
RADIUS = 4.0  # GLGizmoBrimEars::get_brim_default_radius : 0.5 × 16 / 2

def mat(t):
    v = [float(x) for x in t.split()]
    # 3MF : m00 m01 m02 m10 m11 m12 m20 m21 m22 tx ty tz (lignes = vecteurs de base)
    return [[v[0], v[3], v[6], v[9]], [v[1], v[4], v[7], v[10]], [v[2], v[5], v[8], v[11]]]

def apply(m, p):
    return [m[r][0] * p[0] + m[r][1] * p[1] + m[r][2] * p[2] + m[r][3] for r in range(3)]

_base_dir = tempfile.TemporaryDirectory()
_r = subprocess.run(make_refs.orca_cmd() + ["--export-3mf", "base.3mf", "--outputdir", _base_dir.name, str(SRC)],
                    capture_output=True, text=True, timeout=600)
if _r.returncode != 0:
    raise SystemExit(f"échec du réexport ({_r.returncode}) : {(_r.stdout + _r.stderr)[-400:]}")
src = zipfile.ZipFile(Path(_base_dir.name) / "base.3mf")
model = src.read("3D/3dmodel.model").decode()
comp = mat(re.search(r'cube20\.stl_1\.model" objectid="1"[^>]*transform="([^"]+)"', model).group(1))
item = mat(re.search(r'<item objectid="2"[^>]*transform="([^"]+)"', model).group(1))
sub = src.read("3D/Objects/cube20.stl_1.model").decode()
verts = [apply(comp, [float(a), float(b), float(c)])
         for a, b, c in re.findall(r'<vertex x="([^"]+)" y="([^"]+)" z="([^"]+)"', sub)]
lo = [min(v[i] for v in verts) for i in range(3)]
hi = [max(v[i] for v in verts) for i in range(3)]
# Rotation identité pour le cube (vérifié) : local = monde - translation.
assert all(abs(item[r][c] - (1.0 if r == c else 0.0)) < 1e-9 for r in range(3) for c in range(3))
z_local = -0.0001 - item[2][3]
points = []
for x in (lo[0], hi[0]):
    for y in (lo[1], hi[1]):
        points.append((x, y, z_local, RADIUS))
ears = "brim_points_format_version=0\nobject_id=1|" + " ".join(
    "%f %f %f %f" % p for p in points) + "\n"
with zipfile.ZipFile(DEST, "w", zipfile.ZIP_DEFLATED) as z:
    for i in src.infolist():
        d = src.read(i.filename)
        if i.filename == "Metadata/project_settings.config":
            cfg = json.loads(d)
            cfg["brim_type"] = "painted"
            d = json.dumps(cfg, indent=4, ensure_ascii=False).encode()
        z.writestr(i, d)
    z.writestr("Metadata/brim_ear_points.txt", ears)
with tempfile.TemporaryDirectory() as tmp:
    out = Path(tmp)
    r = subprocess.run(make_refs.orca_cmd() + ["--slice", "0", "--outputdir", str(out), str(DEST)],
                       capture_output=True, text=True, timeout=900)
    g = sorted(out.glob("*.gcode"))
    if r.returncode != 0 or not g:
        raise SystemExit(f"échec du CLI ({r.returncode}) : {(r.stdout + r.stderr)[-400:]}")
    shutil.copy(g[0], HERE / "brim-ears.orca.gcode")
print("brim-ears", len(points), "oreilles", ears.strip().splitlines()[1][:80])
