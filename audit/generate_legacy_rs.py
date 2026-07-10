#!/usr/bin/env python3
"""Génère engine/src/params/legacy_tables.rs depuis audit/legacy_keys.json (T035).

Seules les conversions **inconditionnelles et purement tabulaires** sont
générées ici : les renommages simples (clé → clé) et l'ensemble des clés
obsolètes ignorées. Les transformations conditionnées par la valeur (dont les
conditions ont été retirées de l'audit — seul l'assignement + son SHA-1 sont
conservés pour la détection de dérive) sont portées à la main dans
`engine/src/params/legacy.rs`, fidèlement à `PrintConfigDef::handle_legacy`.

Le fichier généré est committé ; sa fraîcheur est verrouillée par un test du
crate engine comparant le SHA-1 de legacy_keys.json au constant embarqué.
"""

from __future__ import annotations

import hashlib
import json

from common import AUDIT_DIR, REPO_ROOT

OUT = REPO_ROOT / "engine" / "src" / "params" / "legacy_tables.rs"


def rs_str(s: str) -> str:
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'


def main() -> None:
    src = (AUDIT_DIR / "legacy_keys.json").read_bytes()
    data = json.loads(src)

    renames = data["renames"]
    ignored = sorted(data["ignored"])

    rename_rows = ",\n".join(
        f"    ({rs_str(old)}, {rs_str(renames[old])})" for old in sorted(renames)
    )
    ignored_rows = ",\n".join(f"    {rs_str(k)}" for k in ignored)

    sha1 = hashlib.sha1(src).hexdigest()
    body = f"""// GÉNÉRÉ par audit/generate_legacy_rs.py — NE PAS ÉDITER (constitution V).
// Source : audit/legacy_keys.json (renommages {len(renames)}, ignorées {len(ignored)}).
// Fraîcheur verrouillée par le test `legacy_tables_synced_with_audit`.

/// SHA-1 de audit/legacy_keys.json au moment de la génération.
pub const SOURCE_SHA1: &str = "{sha1}";

/// Renommages inconditionnels (ancienne clé → nouvelle clé), triés par ancienne
/// clé pour la recherche dichotomique.
pub static RENAMES: &[(&str, &str)] = &[
{rename_rows}
];

/// Clés obsolètes explicitement ignorées à l'import (triées).
pub static IGNORED: &[&str] = &[
{ignored_rows}
];
"""
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(body, encoding="utf-8")
    print(f"{OUT}: {len(renames)} renommages, {len(ignored)} ignorées, sha1 {sha1[:12]}")


if __name__ == "__main__":
    main()
