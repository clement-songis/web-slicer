#!/usr/bin/env python3
"""Extrait la table de conversion des clés legacy de PrintConfigDef.

Sources parsées dans vendor/OrcaSlicer/src/libslic3r/PrintConfig.cpp :
  - `handle_legacy(opt_key, value)`  : renommages, suppressions, transformations
    de valeur (chaîne if/else-if) + set `ignore` des clés obsolètes ;
  - `handle_legacy_composite(config)` : conversions composites (plusieurs clés
    fusionnées), capturées avec le hash du corps pour détecter toute dérive
    du vendor lors des mises à jour.

Sortie : audit/legacy_keys.json — consommée par le codegen
`engine/src/params/legacy.rs` (T035). Les transformations de valeur non
mécanisables portent leur extrait C++ (snippet) : le portage Rust est manuel
mais tracé, et un changement upstream casse le hash.
"""

from __future__ import annotations

import hashlib
import re

from common import find_matching_paren, read_source, strip_comments, write_json

SOURCE = "src/libslic3r/PrintConfig.cpp"


def find_function_body(code: str, signature_re: str) -> str:
    m = re.search(signature_re, code)
    if not m:
        raise SystemExit(f"fonction introuvable : {signature_re}")
    open_brace = code.index("{", m.end() - 1)
    close = find_brace_close(code, open_brace)
    return code[open_brace + 1 : close]


def find_brace_close(text: str, open_pos: int) -> int:
    return find_matching_paren(text, open_pos, "{", "}")


def split_if_chain(body: str):
    """Découpe la chaîne if / else if de premier niveau en (condition, bloc)."""
    entries = []
    i = 0
    n = len(body)
    while i < n:
        m = re.compile(r"(?:else\s+)?if\s*\(").search(body, i)
        if not m:
            break
        cond_close = find_matching_paren(body, body.index("(", m.start()))
        cond = body[body.index("(", m.start()) + 1 : cond_close]
        brace = body.index("{", cond_close)
        block_close = find_brace_close(body, brace)
        block = body[brace + 1 : block_close]
        entries.append((cond, block))
        # continue après le bloc : les chaînes if/else-if successives du corps
        # sont toutes parcourues (les if imbriqués, sautés avec le bloc, sont
        # capturés via le snippet de leur bloc parent)
        i = block_close + 1
    return entries


def main() -> None:
    code = strip_comments(read_source(SOURCE))

    body = find_function_body(
        code, r"void\s+PrintConfigDef::handle_legacy\s*\(\s*t_config_option_key"
    )

    renames: dict[str, str] = {}
    erased: list[dict] = []
    value_transforms: list[dict] = []

    for cond, block in split_if_chain(body):
        old_keys = re.findall(r'opt_key\s*==\s*"([^"]+)"', cond)
        if not old_keys:
            continue
        cond_on_value = "value" in cond
        assigns = re.findall(r'opt_key\s*=\s*"([^"]*)"', block)
        new_key = assigns[-1] if assigns else None
        value_changed = bool(
            re.search(r"\bvalue\s*[.=]|ReplaceString\s*\(", block)
        )
        snippet = re.sub(r"\s+", " ", block).strip()
        entry_base = {
            "old_keys": old_keys,
            "condition_on_value": cond_on_value,
        }
        if new_key == "":
            erased.append({**entry_base, "reason": "invalidé (opt_key effacée)",
                           "snippet": snippet})
        elif new_key is not None and not value_changed and not cond_on_value:
            for k in old_keys:
                renames[k] = new_key
        else:
            # renommage conditionnel et/ou transformation de valeur :
            # portage manuel tracé par snippet + hash
            value_transforms.append({
                **entry_base,
                "new_key": new_key,
                "value_changed": value_changed,
                "snippet": snippet,
                "snippet_sha1": hashlib.sha1(snippet.encode()).hexdigest(),
            })

    m = re.search(r"static\s+std::set<std::string>\s+ignore\s*=\s*\{(.*?)\};", body, re.S)
    ignored = sorted(re.findall(r'"([^"]+)"', m.group(1))) if m else []

    composite_body = find_function_body(
        code, r"void\s+PrintConfigDef::handle_legacy_composite\s*\(\s*DynamicPrintConfig"
    )
    composite_keys = sorted(set(re.findall(r'"([a-z][a-z0-9_]{2,})"', composite_body)))
    composite = {
        "keys_mentioned": composite_keys,
        "body_sha1": hashlib.sha1(
            re.sub(r"\s+", " ", composite_body).encode()
        ).hexdigest(),
        "note": "conversions composites (fusion de clés) — portage manuel dans "
                "engine::params::legacy, dérive vendor détectée par le hash",
    }

    unknown_note = (
        "Le comportement final de handle_legacy efface toute clé absente du "
        "registre : les clés inconnues des presets vendeurs (fautes "
        "historiques, clés de forks) sont donc ignorées à l'import, à "
        "l'identique d'OrcaSlicer."
    )

    data = {
        "source": SOURCE,
        "generated_by": "audit/extract_legacy_keys.py",
        "summary": {
            "renames": len(renames),
            "erased_rules": len(erased),
            "value_transforms": len(value_transforms),
            "ignored": len(ignored),
            "composite_keys": len(composite_keys),
        },
        "renames": dict(sorted(renames.items())),
        "erased": erased,
        "value_transforms": value_transforms,
        "ignored": ignored,
        "composite": composite,
        "unknown_keys_behavior": unknown_note,
    }
    out = write_json("legacy_keys.json", data)
    print(f"{out}: {data['summary']}")


if __name__ == "__main__":
    main()
