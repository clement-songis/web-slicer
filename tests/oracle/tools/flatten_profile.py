#!/usr/bin/env python3
"""Aplatit un preset système OrcaSlicer (chaîne `inherits` résolue), indépendamment de l'app.

  flatten_profile.py "<nom du preset>" [--profiles vendor/OrcaSlicer/resources/profiles]

Sortie : JSON aplati prêt pour `orca-slicer --load-settings` (from=User, sans inherits).
"""
from __future__ import annotations

import argparse
import json
import sys
from functools import lru_cache
from pathlib import Path

DEFAULT_PROFILES = Path(__file__).resolve().parents[3] / "vendor" / "OrcaSlicer" / "resources" / "profiles"
DROP = {"inherits", "instantiation", "setting_id", "base_id", "filament_id", "renamed_from", "description"}


class Profiles:
    def __init__(self, root: Path):
        self.root = root
        self.index: dict[tuple[str, str], Path] = {}
        for p in root.glob("*/**/*.json"):
            try:
                d = json.loads(p.read_text(encoding="utf-8"))
            except Exception:
                continue
            if isinstance(d, dict) and "name" in d and d.get("type") in ("machine", "process", "filament"):
                vendor = p.relative_to(root).parts[0]
                self.index.setdefault((vendor, d["name"]), p)

    def find(self, name: str, vendor: str | None = None) -> tuple[str, Path]:
        if vendor and (vendor, name) in self.index:
            return vendor, self.index[(vendor, name)]
        for pref in ("OrcaFilamentLibrary", None):
            for (v, n), p in self.index.items():
                if n == name and (pref is None or v == pref):
                    return v, p
        close = sorted({n for (_, n) in self.index if name.split(" ")[0].lower() in n.lower()})[:15]
        raise SystemExit(f"preset introuvable : {name!r}. Proches : {close}")

    @lru_cache(maxsize=None)
    def resolve(self, name: str, vendor: str | None = None) -> dict:
        v, p = self.find(name, vendor)
        d = json.loads(p.read_text(encoding="utf-8"))
        merged: dict = {}
        if d.get("inherits"):
            merged.update(self.resolve(d["inherits"], v))
        merged.update(d)
        return merged


def flatten(name: str, root: Path = DEFAULT_PROFILES) -> dict:
    d = dict(Profiles(root).resolve(name))
    for k in DROP:
        d.pop(k, None)
    d["name"] = name
    d["from"] = "User"
    return d


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("name")
    ap.add_argument("--profiles", type=Path, default=DEFAULT_PROFILES)
    a = ap.parse_args()
    json.dump(flatten(a.name, a.profiles), sys.stdout, indent=1, ensure_ascii=False)
