#!/usr/bin/env python3
"""Génère le corpus de parité SC-003 : 10 modèles × 5 presets (T022).

Pour chaque combinaison (modèle, preset), le script :
  1. **aplatit** les chaînes d'héritage OrcaSlicer (machine/filament/process)
     — `--load-settings` ne résout PAS `inherits`, il faut donc résoudre nous-
     mêmes `inherits` dans `$ORCA_SRC/resources/profiles/*` (enfant écrase parent) ;
  2. exporte via `orca-slicer` un **3MF au config résolu** (toutes les clés de
     tranchage embarquées) ;
  3. tranche ce 3MF via `orca-slicer` → **référence desktop** (mêmes libslic3r) ;
  4. consigne (couches, temps estimé) dans `parity/references.json`.

Le test `engine/tests/gcode_parity.rs` (FFI) tranche ensuite chaque 3MF committé
et compare aux métriques de référence — sans dépendre du CLI orca au moment du test.

Prérequis : devShell (`orca-slicer` dans le PATH, `$ORCA_SRC` défini).
Ré-exécutable : `python3 engine/tests/fixtures/generate_parity_corpus.py`.
Les combinaisons qu'orca lui-même refuse de trancher sont consignées dans le
rapport (et dans `specs/.../exclusions.md`), jamais omises silencieusement.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
OUT = HERE / "parity"
PROFILES = Path(os.environ["ORCA_SRC"]) / "resources" / "profiles"
VENDORS = ["BBL", "OrcaFilamentLibrary", "Voron", "OrcaArena"]

# Clés méta non propagées comme réglages de tranchage.
META = {"inherits", "from", "type", "instantiation", "setting_id"}

# Plaque texturée PEI : compatible PLA/PETG/ABS — évite le refus « Cool Plate
# does not support filament » (CLI_FILAMENT_NOT_MATCH_BED_TYPE) sur PETG/ABS.
BED_TYPE = "Textured PEI Plate"


def build_index() -> dict:
    """(kind, name) -> Path, sur tous les vendeurs (premier gagne)."""
    idx: dict = {}
    for vendor in VENDORS:
        for sub in ("process", "filament", "machine"):
            d = PROFILES / vendor / sub
            if not d.is_dir():
                continue
            for f in d.glob("*.json"):
                try:
                    nm = json.loads(f.read_text()).get("name")
                except Exception:
                    continue
                if nm:
                    idx.setdefault((sub, nm), f)
    return idx


def flatten(idx: dict, sub: str, name: str) -> dict:
    """Chaîne leaf→root aplatie ; l'enfant écrase le parent (comme OrcaSlicer)."""
    chain, seen, cur = [], set(), name
    while cur and cur not in seen:
        seen.add(cur)
        f = idx.get((sub, cur))
        if not f:
            raise KeyError(f"preset {sub} introuvable : {cur!r}")
        d = json.loads(f.read_text())
        chain.append(d)
        cur = d.get("inherits")
    merged: dict = {}
    for d in reversed(chain):  # root d'abord, leaf en dernier
        for k, v in d.items():
            if k not in META:
                merged[k] = v
    return merged


def resolved_settings(idx: dict, combo: dict) -> tuple[Path, Path, Path]:
    """Écrit les 3 configs plats (machine/filament/process) et renvoie leurs chemins."""
    fil_name = combo["filament"]
    proc = flatten(idx, "process", combo["process"])
    fil = flatten(idx, "filament", fil_name)
    mach = flatten(idx, "machine", combo["machine"])
    proc["curr_bed_type"] = BED_TYPE
    files = {}
    for d, nm, ty in [
        (proc, combo["process"], "process"),
        (fil, fil_name, "filament"),
        (mach, combo["machine"], "machine"),
    ]:
        # from="system" => new_printer_system_name = nom machine ; le gate de
        # compat process (compatible_printers) référence ce nom sans héritage vivant.
        d["name"], d["type"], d["from"] = nm, ty, "system"
        p = OUT / f".settings-{ty}.json"
        p.write_text(json.dumps(d))
        files[ty] = p
    return files["process"], files["filament"], files["machine"]


def run_orca(args: list[str]) -> subprocess.CompletedProcess:
    # cwd=OUT : orca écrit `result.json` dans son répertoire courant ; on le
    # cantonne au dossier du corpus (nettoyé ensuite) plutôt qu'à la racine du dépôt.
    return subprocess.run(["orca-slicer", *args], capture_output=True, text=True, cwd=OUT)


def _duration_s(frag: str) -> float:
    """`1h 2m 3s` / `29m 56s` / `45s` → secondes."""
    total = 0.0
    for tok in frag.split():
        if tok and tok[-1] in "hms" and tok[:-1].replace(".", "", 1).isdigit():
            total += float(tok[:-1]) * {"h": 3600, "m": 60, "s": 1}[tok[-1]]
        else:
            break
    return total


def header_metrics(gcode: Path):
    """(couches, temps estimé s, filament longueur mm) depuis l'en-tête OrcaSlicer.

    Deux saveurs de G-code : Bambu (`total estimated time:`) et Marlin/Klipper
    (`estimated printing time (normal mode) =`, ex. profils Voron). La longueur
    de filament (`filament used [mm]`) est la **métrique de parité de tranchage**
    indépendante du build (le temps estimé, lui, dépend de la représentation
    arc-fitting propre à chaque build de libslic3r)."""
    layers, time_s, filament_mm = None, None, None
    with gcode.open(errors="replace") as f:
        for line in f:
            if "total layer number:" in line:
                layers = int(line.split("total layer number:")[1].strip())
            elif "total estimated time:" in line:
                time_s = _duration_s(line.split("total estimated time:")[1])
            elif "estimated printing time (normal mode) =" in line:
                time_s = _duration_s(line.split("=", 1)[1])
            elif "filament used [mm]" in line:
                filament_mm = float(line.split("=")[1].strip())
            if layers is not None and time_s is not None and filament_mm is not None:
                break
    return layers, time_s, filament_mm


def main() -> None:
    OUT.mkdir(exist_ok=True)
    manifest = json.loads((HERE / "manifest.json").read_text())
    models = [m["file"] for m in manifest["models"] if m["file"].endswith(".stl")]
    combos = manifest["preset_combos"]
    idx = build_index()

    references: dict = {}
    failures: list = []
    for combo in combos:
        pf, ff, mf = resolved_settings(idx, combo)
        for model in models:
            case = f"{Path(model).stem}__{combo['id']}"
            tmf = OUT / f"{case}.3mf"
            if tmf.exists():
                tmf.unlink()
            exp = run_orca(["--load-settings", f"{pf};{mf}", "--load-filaments",
                            str(ff), "--export-3mf", str(tmf), str(HERE / model)])
            if not tmf.exists():
                failures.append((case, "export", exp.returncode,
                                 (exp.stderr or exp.stdout).strip()[:120]))
                continue
            for g in OUT.glob("*.gcode"):
                g.unlink()
            sl = run_orca(["--slice", "0", "--outputdir", str(OUT), str(tmf)])
            gcs = list(OUT.glob("*.gcode"))
            if not gcs:
                failures.append((case, "slice", sl.returncode,
                                 (sl.stderr or sl.stdout).strip()[:120]))
                tmf.unlink(missing_ok=True)
                continue
            layers, time_s, filament_mm = header_metrics(gcs[0])
            gcs[0].unlink()
            if layers is None or time_s is None or filament_mm is None:
                failures.append((case, "metrics", 0, "en-tête incomplet (couches/temps/filament)"))
                tmf.unlink(missing_ok=True)
                continue
            references[case] = {
                "model": model, "preset_id": combo["id"],
                "layers": layers, "time_s": time_s, "filament_mm": filament_mm,
            }
            print(f"OK  {case:40} {layers} c. / {time_s:.0f}s / {filament_mm:.1f}mm")

    # Nettoie les configs temporaires + l'artefact `result.json` d'orca CLI
    # (cantonné à OUT via cwd) pour ne rien committer d'inutile.
    for p in OUT.glob(".settings-*.json"):
        p.unlink()
    (OUT / "result.json").unlink(missing_ok=True)

    (OUT / "references.json").write_text(
        json.dumps({"generated_by": Path(__file__).name,
                    "bed_type": BED_TYPE,
                    "cases": references}, indent=2, ensure_ascii=False) + "\n")

    print(f"\n{len(references)} cas OK / {len(models) * len(combos)} tentés")
    if failures:
        print(f"\n{len(failures)} échecs (orca refuse — à consigner en exclusions) :")
        for case, phase, rc, msg in failures:
            print(f"  FAIL {case:40} [{phase} rc={rc}] {msg}")


if __name__ == "__main__":
    main()
