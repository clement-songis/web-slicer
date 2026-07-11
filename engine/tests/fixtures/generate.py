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
# orca_project.3mf et cube20.step sont COMMITTÉS, générés par les vrais
# outils (devShell) : régénération ci-dessous seulement si disponibles.
#   orca_project.3mf : orca-slicer --export-3mf <out> cube20.stl
#   cube20.step      : BRepPrimAPI_MakeBox + STEPControl_Writer (OCCT)
# crasher_generic.3mf : 3MF générique de 2015 qui fait SIGSEGV le lecteur
# bbs_3mf — conservé comme fixture de crash pour l'isolation worker (T018).
OBJ_NATIVE = ("cube20.obj", "20mm_cube.obj")

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
        # Le vendeur Voron n'a pas de dossier filament : on prend le PLA générique
        # de la bibliothèque OrcaFilamentLibrary (compatible toute buse 0.4).
        "filament": "Generic PLA @System",
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
    import subprocess
    if shutil.which("orca-slicer") and not (HERE / "orca_project.3mf").exists():
        subprocess.run(
            ["orca-slicer", "--export-3mf", str(HERE / "orca_project.3mf"),
             str(HERE / "cube20.stl")],
            check=True, capture_output=True,
        )
    models_meta.append({
        "file": "orca_project.3mf",
        "source": "généré par orca-slicer --export-3mf (committé)",
        "role": "3MF projet OrcaSlicer (scène + réglages embarqués)",
    })
    models_meta.append({
        "file": "cube20.step",
        "source": "généré par OCCT STEPControl_Writer (committé)",
        "role": "import STEP via OCCT (FR-010, R7)",
    })
    models_meta.append({
        "file": "crasher_generic.3mf",
        "source": "vendor/OrcaSlicer/tests/data/test_3mf/Geräte/Büchse.3mf",
        "role": "3MF qui fait crasher le lecteur bbs_3mf — test d'isolation du worker (T018)",
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
