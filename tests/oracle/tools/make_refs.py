#!/usr/bin/env python3
"""Produit les références de parité avec le vrai OrcaSlicer (CLI). Lancé par un humain ou la boucle.

  make_refs.py [--only base,golden,params] [--keys k1,k2] [--jobs 4] [--force]

Écrit dans tests/oracle/refs/ :
  base_config.json, base.gcode            preset de base (harness/oracle.json) sur le modèle de test
  golden/<id>.gcode + .config.json        corpus modèles × process
  param/<clé>.gcode | param/<clé>.error   un tranchage par paramètre modifié
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

HERE = Path(__file__).resolve().parent
ORACLE = HERE.parent
REPO = ORACLE.parent.parent
REFS = ORACLE / "refs"
sys.path.insert(0, str(HERE))
from flatten_profile import DEFAULT_PROFILES, DROP, Profiles  # noqa: E402

CFG = json.loads((REPO / "harness" / "oracle.json").read_text())
META = {"type", "name", "from", "version", "is_custom_defined", "instantiation"}
PARAMS = json.loads((REPO / "audit" / "parameters.json").read_text())["parameters"]


def _scalar(v):
    return v[0] if isinstance(v, list) and v else v


def distinct_value(key: str, value, current):
    """Garantit une valeur de test ≠ valeur du preset de base (sinon le test ne prouve rien)."""
    if current is None or str(_scalar(value)) != str(_scalar(current)):
        return value
    d = PARAMS.get(key, {})
    t, cur = d.get("type", ""), str(_scalar(current))
    vec = isinstance(value, list)
    lo, hi = (x if isinstance(x, (int, float)) else None for x in (d.get("min"), d.get("max")))
    cands: list[str] = []
    if "Bool" in t:
        cands = ["0" if cur in ("1", "true") else "1"]
    elif "Enum" in t:
        cands = [e for e in d.get("enum_values") or [] if e != cur]
    else:
        pct = cur.endswith("%")
        try:
            n = float(cur.rstrip("%"))
        except ValueError:
            return value
        is_int = "Int" in t or pct
        for c in (n + 1, n - 1, n * 1.25 or 1, n * 0.75) if is_int else (n * 1.25 or 1, n * 0.75, n + 1, n - 0.5):
            if (lo is None or c >= lo) and (hi is None or c <= hi) and c != n:
                v = str(int(round(c))) if is_int else f"{c:.4f}".rstrip("0").rstrip(".")
                cands.append(v + ("%" if pct else ""))
    if not cands:
        return value
    return [cands[0]] if vec else cands[0]


def orca_cmd() -> list[str]:
    exe = os.environ.get("ORCA_SLICER") or shutil.which("orca-slicer") or shutil.which("OrcaSlicer")
    if not exe:
        raise SystemExit("orca-slicer introuvable (nix develop le fournit, ou définir ORCA_SLICER)")
    if not os.environ.get("DISPLAY") and shutil.which("xvfb-run"):
        return ["xvfb-run", "-a", exe]
    return [exe]


class Refs:
    def __init__(self, profiles_root: Path):
        self.p = Profiles(profiles_root)

    def preset(self, name: str) -> dict:
        d = dict(self.p.resolve(name))
        for k in DROP:
            d.pop(k, None)
        d["name"], d["from"] = name, "User"
        return d

    def trio(self, process: str | None = None) -> tuple[dict, dict, dict]:
        return self.preset(CFG["printer"]), self.preset(process or CFG["process"]), self.preset(CFG["filament"])


def merged(printer: dict, process: dict, filament: dict) -> dict:
    out: dict = {}
    for d in (printer, filament, process):
        out.update({k: v for k, v in d.items() if k not in META})
    out["printer_settings_id"] = printer["name"]
    out["print_settings_id"] = process["name"]
    out["filament_settings_id"] = [filament["name"]]
    return out


def run_orca(printer: dict, process: dict, filament: dict, model: str, dest: Path) -> bool:
    """Tranche avec Orca ; écrit dest (.gcode) ou dest.with_suffix('.error')."""
    with tempfile.TemporaryDirectory() as tmp:
        t = Path(tmp)
        for n, d in (("machine", printer), ("process", process), ("filament", filament)):
            (t / f"{n}.json").write_text(json.dumps(d, ensure_ascii=False))
        out = t / "out"
        out.mkdir()
        model_path = t / f"{model}.stl"
        shutil.copy(ORACLE / "fixtures" / f"{model}.stl", model_path)
        cmd = orca_cmd() + ["--load-settings", f"{t/'machine.json'};{t/'process.json'}",
                            "--load-filaments", str(t / "filament.json"),
                            "--arrange", "1", "--orient", "0", "--slice", "0",
                            "--outputdir", str(out), str(model_path)]
        try:
            r = subprocess.run(cmd, capture_output=True, text=True, timeout=600)
            log = r.stdout + r.stderr
        except subprocess.TimeoutExpired:
            log = "timeout"
        gcodes = sorted(out.glob("*.gcode"))
        dest.parent.mkdir(parents=True, exist_ok=True)
        if gcodes:
            shutil.copy(gcodes[0], dest)
            dest.with_suffix(".error").unlink(missing_ok=True)
            return True
        res = out / "result.json"
        why = json.loads(res.read_text()).get("error_string", "") if res.exists() else ""
        dest.with_suffix(".error").write_text(f"{why}\n{log[-1500:]}")
        return False


def place(key: str, scope: str | None, trio: tuple[dict, dict, dict]) -> dict:
    printer, process, filament = trio
    for d in (process, filament, printer):
        if key in d:
            return d
    return {"printer": printer, "filament": filament}.get(scope or "", process)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--only", default="base,golden,params")
    ap.add_argument("--keys", default="")
    ap.add_argument("--jobs", type=int, default=os.cpu_count() or 4)
    ap.add_argument("--force", action="store_true")
    ap.add_argument("--profiles", type=Path, default=DEFAULT_PROFILES)
    a = ap.parse_args()
    only = set(a.only.split(","))
    refs = Refs(a.profiles)
    jobs = []

    if "base" in only:
        trio = refs.trio()
        REFS.mkdir(parents=True, exist_ok=True)
        (REFS / "base_config.json").write_text(json.dumps(merged(*trio), indent=1, ensure_ascii=False))
        jobs.append((trio, CFG["param_model"], REFS / "base.gcode"))

    if "golden" in only:
        for c in json.loads((ORACLE / "golden" / "cases.json").read_text()):
            name = c["id"].removeprefix("golden.")
            trio = refs.trio(c["process"])
            (REFS / "golden").mkdir(parents=True, exist_ok=True)
            (REFS / "golden" / f"{name}.config.json").write_text(json.dumps(merged(*trio), indent=1, ensure_ascii=False))
            jobs.append((trio, c["model"], REFS / "golden" / f"{name}.gcode"))

    if "params" in only:
        keys = set(filter(None, a.keys.split(",")))
        for c in json.loads((ORACLE / "params" / "cases.json").read_text()):
            if c["value"] is None or (keys and c["key"] not in keys):
                continue
            req = ORACLE / "params" / "requires" / f"{c['key']}.json"
            if req.exists():
                c["requires"] = json.loads(req.read_text()).get("requires", {})
            trio = tuple(json.loads(json.dumps(x)) for x in refs.trio())
            target = place(c["key"], c.get("scope"), trio)
            value = distinct_value(c["key"], c["value"], target.get(c["key"]))
            for k, v in {**(c.get("requires") or {}), c["key"]: value}.items():
                place(k, c.get("scope"), trio)[k] = v
            (REFS / "param").mkdir(parents=True, exist_ok=True)
            (REFS / "param" / f"{c['key']}.value.json").write_text(json.dumps(value))
            jobs.append((trio, CFG["param_model"], REFS / "param" / f"{c['key']}.gcode"))

    todo = [j for j in jobs if a.force or not (j[2].exists() or j[2].with_suffix(".error").exists())]
    print(f"{len(todo)} tranchages Orca à faire ({len(jobs) - len(todo)} déjà présents)")
    ok = 0
    with ThreadPoolExecutor(a.jobs) as ex:
        for i, good in enumerate(ex.map(lambda j: run_orca(j[0][0], j[0][1], j[0][2], j[1], j[2]), todo), 1):
            ok += good
            if i % 25 == 0 or i == len(todo):
                print(f"  {i}/{len(todo)} ({ok} ok)")
    errs = sorted(p.name for p in REFS.rglob("*.error"))
    if errs:
        print(f"{len(errs)} refus d'Orca (→ needs_review) : {', '.join(errs[:20])}")
    if not (REFS / "base.gcode").exists():
        sys.exit("ÉCHEC : la référence de base n'a pas pu être produite — vérifier harness/oracle.json")


if __name__ == "__main__":
    main()
