#!/usr/bin/env python3
"""Génère les références Orca sans modifier les outils oracle gelés.

`tests/oracle/tools/make_refs.py` aplatit des presets utilisateur mais retire
`inherits`. Orca CLI utilise ce champ pour retrouver le nom système lors de sa
validation imprimante/process : sans lui, même un trio compatible est refusé.
Ce lanceur conserve l'aplatissement et restaure le parent système déclaré.
"""
from __future__ import annotations

import importlib.util
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SOURCE = REPO / "tests/oracle/tools/make_refs.py"


def main() -> None:
    spec = importlib.util.spec_from_file_location("web_slicer_oracle_refs", SOURCE)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"impossible de charger {SOURCE}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)

    def preset_with_system_inheritance(self, name: str) -> dict:
        preset = self.p.resolve(name)
        for key in module.DROP:
            preset.pop(key, None)
        preset["name"] = name
        preset["from"] = "User"
        # Le profil est bien une copie utilisateur aplatie du profil système.
        preset["inherits"] = name
        return preset

    module.Refs.preset = preset_with_system_inheritance
    module.main()


if __name__ == "__main__":
    main()
