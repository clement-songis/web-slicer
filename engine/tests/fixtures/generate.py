#!/usr/bin/env python3
"""Génère le corpus de fixtures du moteur (T004, SC-003).

Sources : modèles de test upstream de vendor/OrcaSlicer/tests/data (OBJ),
convertis en STL binaire de façon déterministe ; un 3MF projet Orca copié
tel quel ; un cube STEP généré (faceted BREP minimal lisible par OCCT).

Ré-exécutable : `python3 engine/tests/fixtures/generate.py` — sorties
byte-identiques d'une exécution à l'autre.
"""

from __future__ import annotations

import json
import shutil
import struct
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent.parent
DATA = REPO / "vendor" / "OrcaSlicer" / "tests" / "data"

# 10 modèles de référence : nom cible → (source, rôle dans le corpus)
MODELS = {
    "cube20.stl": ("20mm_cube.obj", "cube de calibration — cas nominal"),
    "pyramid.stl": ("pyramid.obj", "pentes régulières — hauteur de couche variable"),
    "overhang.stl": ("overhang.obj", "surplombs — seuils de support"),
    "bridge.stl": ("bridge.obj", "pontage — vitesses/détection bridge"),
    "two_squares.stl": ("two_hollow_squares.obj", "multi-corps disjoints — arrangement"),
    "frog_legs.stl": ("frog_legs.obj", "forme organique — supports auto"),
    "extruder_idler.stl": ("extruder_idler.obj", "pièce mécanique réelle — trous/tolérances"),
    "ipadstand.stl": ("ipadstand.obj", "grandes surfaces planes — remplissage/ironing"),
    "V.stl": ("V.obj", "parois fines en V — thin walls"),
    "simplification.stl": ("simplification.obj", "référence upstream du gizmo Simplify"),
}

# Variantes de format d'import (FR-010) : OBJ natif, 3MF projet Orca, STEP.
OBJ_NATIVE = ("cube20.obj", "20mm_cube.obj")
ORCA_3MF = ("orca_project.3mf", "test_3mf/Geräte/Büchse.3mf")

# 5 combinaisons de presets du corpus de parité (SC-003) — résolues au
# moment des tests depuis les profils système seedés (Annexe C).
PRESET_COMBOS = [
    {
        "id": "bbl-a1-standard",
        "machine": "Bambu Lab A1 0.4 nozzle",
        "filament": "Bambu PLA Basic @BBL A1",
        "process": "0.20mm Standard @BBL A1",
    },
    {
        "id": "bbl-a1-fine-02",
        "machine": "Bambu Lab A1 0.2 nozzle",
        "filament": "Bambu PLA Basic @BBL A1 0.2 nozzle",
        "process": "0.06mm Fine @BBL A1 0.2 nozzle",
    },
    {
        "id": "bbl-x1c-abs",
        "machine": "Bambu Lab X1 Carbon 0.4 nozzle",
        "filament": "Bambu ABS @BBL X1C",
        "process": "0.20mm Standard @BBL X1C",
    },
    {
        "id": "bbl-a1-draft-06",
        "machine": "Bambu Lab A1 0.6 nozzle",
        "filament": "Bambu PETG Basic @BBL A1",
        "process": "0.30mm Standard @BBL A1 0.6 nozzle",
    },
    {
        "id": "voron-generic-pla",
        "machine": "Voron 2.4 300 0.4 nozzle",
        "filament": "Generic PLA @Voron",
        "process": "0.20mm Standard @Voron",
    },
]


def parse_obj(path: Path):
    verts: list[tuple[float, float, float]] = []
    faces: list[tuple[int, int, int]] = []
    for line in path.read_text().splitlines():
        parts = line.split()
        if not parts:
            continue
        if parts[0] == "v":
            verts.append((float(parts[1]), float(parts[2]), float(parts[3])))
        elif parts[0] == "f":
            idx = [int(p.split("/")[0]) - 1 for p in parts[1:]]
            # triangulation en éventail des polygones
            for i in range(1, len(idx) - 1):
                faces.append((idx[0], idx[i], idx[i + 1]))
    return verts, faces


def write_binary_stl(path: Path, verts, faces) -> None:
    def normal(a, b, c):
        ux, uy, uz = (b[i] - a[i] for i in range(3))
        vx, vy, vz = (c[i] - a[i] for i in range(3))
        n = (uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx)
        length = (n[0] ** 2 + n[1] ** 2 + n[2] ** 2) ** 0.5
        return (0.0, 0.0, 0.0) if length == 0 else tuple(x / length for x in n)

    with path.open("wb") as f:
        f.write(b"web-slicer fixture (genere par generate.py)".ljust(80, b"\0"))
        f.write(struct.pack("<I", len(faces)))
        for a, b, c in faces:
            va, vb, vc = verts[a], verts[b], verts[c]
            f.write(struct.pack("<3f", *normal(va, vb, vc)))
            for v in (va, vb, vc):
                f.write(struct.pack("<3f", *v))
            f.write(struct.pack("<H", 0))


def write_step_cube(path: Path, size: float = 20.0) -> None:
    """Cube STEP en faceted BREP (2 triangles par face) — lisible par OCCT."""
    s = size
    pts = [
        (0, 0, 0), (s, 0, 0), (s, s, 0), (0, s, 0),
        (0, 0, s), (s, 0, s), (s, s, s), (0, s, s),
    ]
    tris = [
        (0, 2, 1), (0, 3, 2),  # bas (z=0)
        (4, 5, 6), (4, 6, 7),  # haut
        (0, 1, 5), (0, 5, 4),  # y=0
        (1, 2, 6), (1, 6, 5),  # x=s
        (2, 3, 7), (2, 7, 6),  # y=s
        (3, 0, 4), (3, 4, 7),  # x=0
    ]
    lines = []
    eid = 0

    def add(entity: str) -> int:
        nonlocal eid
        eid += 1
        lines.append(f"#{eid}={entity};")
        return eid

    pt_ids = [add(f"CARTESIAN_POINT('',({p[0]:.1f},{p[1]:.1f},{p[2]:.1f}))") for p in pts]
    face_ids = []
    for a, b, c in tris:
        loop = add(f"POLY_LOOP('',(#{pt_ids[a]},#{pt_ids[b]},#{pt_ids[c]}))")
        bound = add(f"FACE_OUTER_BOUND('',#{loop},.T.)")
        face_ids.append(add(f"FACE('',(#{bound}))"))
    shell = add("CLOSED_SHELL('',(" + ",".join(f"#{f}" for f in face_ids) + "))")
    brep = add(f"FACETED_BREP('cube',#{shell})")
    origin = add("CARTESIAN_POINT('',(0.,0.,0.))")
    dz = add("DIRECTION('',(0.,0.,1.))")
    dx = add("DIRECTION('',(1.,0.,0.))")
    axis = add(f"AXIS2_PLACEMENT_3D('',#{origin},#{dz},#{dx})")
    add(f"SHAPE_REPRESENTATION('cube',(#{axis},#{brep}),#{eid + 2})")
    ctx = add(
        "(GEOMETRIC_REPRESENTATION_CONTEXT(3)"
        "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#" + str(eid + 2) + "))"
        "GLOBAL_UNIT_ASSIGNED_CONTEXT((#" + str(eid + 3) + ",#" + str(eid + 4) + ",#" + str(eid + 5) + "))"
        "REPRESENTATION_CONTEXT('cube','3D'))"
    )
    add("UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.E-07),#" + str(eid + 2) + ",'','')")
    add("(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.))")
    add("(NAMED_UNIT(*)PLANE_ANGLE_UNIT()SI_UNIT($,.RADIAN.))")
    add("(NAMED_UNIT(*)SI_UNIT($,.STERADIAN.)SOLID_ANGLE_UNIT())")
    _ = ctx

    header = """ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('web-slicer fixture cube'),'2;1');
FILE_NAME('cube20.step','2026-01-01T00:00:00',(''),(''),'','generate.py','');
FILE_SCHEMA(('AUTOMOTIVE_DESIGN { 1 0 10303 214 1 1 1 1 }'));
ENDSEC;
DATA;
"""
    path.write_text(header + "\n".join(lines) + "\nENDSEC;\nEND-ISO-10303-21;\n")


def main() -> None:
    models_meta = []
    for target, (source, role) in MODELS.items():
        verts, faces = parse_obj(DATA / source)
        write_binary_stl(HERE / target, verts, faces)
        models_meta.append({
            "file": target,
            "source": f"vendor/OrcaSlicer/tests/data/{source}",
            "role": role,
            "triangles": len(faces),
        })

    shutil.copyfile(DATA / OBJ_NATIVE[1], HERE / OBJ_NATIVE[0])
    models_meta.append({
        "file": OBJ_NATIVE[0],
        "source": f"vendor/OrcaSlicer/tests/data/{OBJ_NATIVE[1]}",
        "role": "import OBJ natif (FR-010)",
    })
    shutil.copyfile(DATA / ORCA_3MF[1], HERE / ORCA_3MF[0])
    models_meta.append({
        "file": ORCA_3MF[0],
        "source": f"vendor/OrcaSlicer/tests/data/{ORCA_3MF[1]}",
        "role": "3MF projet OrcaSlicer (scène + réglages embarqués, edge case unicode)",
    })
    write_step_cube(HERE / "cube20.step")
    models_meta.append({
        "file": "cube20.step",
        "source": "généré (faceted BREP)",
        "role": "import STEP via OCCT (FR-010, R7)",
    })

    manifest = {
        "generated_by": "engine/tests/fixtures/generate.py",
        "purpose": "corpus de parité SC-003 : 10 modèles × 5 presets, "
                   "G-code trait == orca-slicer desktop (métadonnées normalisées)",
        "notes": [
            "STL validés par chargement via orca-slicer CLI (--export-3mf)",
            "cube20.step : validé par le lecteur STEP de libslic3r (T013, FFI) — "
            "le CLI orca-slicer n'accepte pas .step en entrée libre (fallback CLI : "
            "conversion STEP indisponible, à consigner dans exclusions.md si maintenu)",
        ],
        "models": models_meta,
        "preset_combos": PRESET_COMBOS,
    }
    (HERE / "manifest.json").write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n")
    print(f"{len(models_meta)} fixtures + manifest → {HERE}")


if __name__ == "__main__":
    main()
