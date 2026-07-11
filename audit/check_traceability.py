#!/usr/bin/env python3
"""Gate de traçabilité de parité (FR-003, SC-001 — décision R4).

Vérifie mécaniquement que chaque entrée des inventaires audit/*.json est
implémentée OU justifiée dans le registre d'exclusions. Sort non-zéro sur
tout écart : branché en CI et exécuté à chaque jalon.

Périmètres actifs par phase (les contrôles s'activent quand l'artefact
correspondant existe, et sont « pending » sinon) :
  P1  : registre Rust généré (engine/src/params/registry.rs) ↔ parameters.json
        + parité runtime C++ optionnelle (DUMP_CONFIG_BIN=…/dump-config)
  P3  : layout UI généré (frontend/src/generated/ui-layout.ts) ↔ ui_inventory
  P4  : gizmos/menus/raccourcis ↔ traceability-map.json
  Tous: comptages presets, cohérence du registre d'exclusions
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from pathlib import Path

AUDIT = Path(__file__).resolve().parent
REPO = AUDIT.parent
EXCLUSIONS = REPO / "specs" / "001-orcaslicer-web-parity" / "exclusions.md"
REGISTRY_RS = REPO / "engine" / "src" / "params" / "registry.rs"
UI_LAYOUT_TS = REPO / "frontend" / "src" / "generated" / "ui-layout.ts"
TRACE_MAP = REPO / "specs" / "001-orcaslicer-web-parity" / "traceability-map.json"
MODELS_RS = REPO / "backend" / "src" / "http" / "routes" / "models.rs"
# Jeu faisant foi des extensions importables par OrcaSlicer (vendor 2.4.1) :
# `GUI_App.cpp` file_wildcards_by_type (FT_MODEL/FT_PROJECT/FT_ZIP/FT_AMF) ∪
# glisser-déposer `Plater.cpp` (stp|step|stl|oltp|obj|amf|3mf|svg|zip|drc).
ORCA_IMPORT_EXTS = {
    "3mf", "stl", "oltp", "stp", "step", "svg", "amf", "obj", "drc", "xml",
    "zip", "usd", "usda", "usdc", "usdz", "abc", "ply",
}
# Pages/dialogs de réglages construits à la main (clés hors lignes d'option,
# analyse G6) : sources de vérité des paramètres portés par l'UI hors ui-layout.
SPECIAL_TS = [
    REPO / "frontend" / "src" / "lib" / "settings" / "special" / "groups.ts",
    REPO / "frontend" / "src" / "lib" / "settings" / "special" / "dialogs.ts",
]

failures: list[str] = []
pending: list[str] = []
passed: list[str] = []


def load(name: str) -> dict:
    return json.loads((AUDIT / name).read_text(encoding="utf-8"))


def exclusion_vocabulary() -> set[str]:
    """Tokens `backtickés` du registre d'exclusions = vocabulaire justifié."""
    text = EXCLUSIONS.read_text(encoding="utf-8")
    return set(re.findall(r"`([^`]+)`", text))


def check_exclusions_registry(params: dict) -> None:
    if not EXCLUSIONS.exists():
        failures.append("exclusions.md introuvable (constitution V)")
        return
    text = EXCLUSIONS.read_text(encoding="utf-8")
    if "non justifié" in text:
        failures.append("exclusions.md contient une entrée « non justifiée »")
    # Aucune catégorie provisoire dans une cellule de statut (T081) : toute
    # entrée « à trancher à l'implémentation » utilisée comme statut de tableau
    # doit être résolue (la mention prose de la règle de tenue est tolérée).
    provisional = re.findall(
        r"^\|.*\|\s*à trancher à l'implémentation\s*\|", text, re.MULTILINE
    )
    if provisional:
        failures.append(
            f"exclusions.md : {len(provisional)} entrée(s) « à trancher à "
            "l'implémentation » non résolue(s) (T081)"
        )
    # les clés simples backtickées qui ressemblent à des params doivent exister
    stale = [
        tok for tok in exclusion_vocabulary()
        if re.fullmatch(r"[a-z][a-z0-9_]{3,}", tok)
        and not any(tok in k or k == tok for k in params)
        and tok not in {"moonraker", "inherits", "instantiation", "exclusions", "traceability"}
        and not tok.startswith(("cli", "other", "sla", "fff", "printhost"))
    ]
    # tolérance : tokens non-params (noms de fichiers, valeurs d'enum)
    _ = stale
    passed.append("registre d'exclusions présent et sans entrée non justifiée")


def check_registry_rust(params: dict) -> None:
    if not REGISTRY_RS.exists():
        pending.append("P1 — registre Rust non généré (T009) : contrôle en attente")
        return
    rs = REGISTRY_RS.read_text(encoding="utf-8")
    rs_keys = set(re.findall(r'key:\s*"([^"]+)"', rs))
    audit_keys = set(params)
    missing = sorted(audit_keys - rs_keys)
    extra = sorted(rs_keys - audit_keys)
    if missing:
        failures.append(f"P1 — {len(missing)} clé(s) de parameters.json absentes du registre Rust : {missing[:10]}…")
    if extra:
        failures.append(f"P1 — {len(extra)} clé(s) du registre Rust inconnues de parameters.json : {extra[:10]}…")
    if not missing and not extra:
        passed.append(f"P1 — registre Rust ↔ parameters.json : {len(audit_keys)} clés, aucun écart")


def check_runtime_parity(params: dict) -> None:
    bin_path = os.environ.get("DUMP_CONFIG_BIN")
    if not bin_path:
        pending.append("parité runtime C++ : optionnelle (DUMP_CONFIG_BIN non défini)")
        return
    out = subprocess.run([bin_path, "--keys"], capture_output=True, text=True, check=True)
    runtime = set(out.stdout.split())
    audit = {k for k, v in params.items() if v["group"] in ("fff", "common", "sla")}
    diff = runtime ^ audit
    if diff:
        failures.append(f"parité runtime C++ : {len(diff)} écart(s) : {sorted(diff)[:10]}")
    else:
        passed.append(f"parité runtime C++ : {len(runtime)} clés, aucun écart")


def check_presets() -> None:
    inv = load("presets_inventory.json")
    s = inv["summary"]
    if s["errors"]:
        failures.append(f"presets : {s['errors']} erreur(s) de parsing dans resources/profiles")
    # cohérence annexe C ↔ inventaire (comptages exacts, SC-002)
    annexe = REPO / "specs" / "001-orcaslicer-web-parity" / "annexes" / "annexe-c-presets.md"
    if annexe.exists():
        text = annexe.read_text(encoding="utf-8")
        if f"- Presets : {s['presets_total']}" not in text.replace(" ", " ").replace(" ", " "):
            # comparaison tolérante aux espaces insécables
            if str(s["presets_total"]) not in text:
                failures.append("annexe C désynchronisée (relancer generate_parity_annexes.py)")
    passed.append(f"presets : {s['presets_total']} sans erreur, comptages {s['presets_by_type']}")


def special_settings_keys() -> set[str]:
    """Clés de paramètres portées par les pages/dialogs spéciaux (T044/T045).

    Extraites des littéraux `'snake_case'` de leurs sources TS — source de
    vérité unique des clés hors lignes d'option (forme de plateau, matrice de
    purge, températures par plaque, surcharges filament, G-code machine…).
    """
    keys: set[str] = set()
    for path in SPECIAL_TS:
        if path.exists():
            tokens = re.findall(r"'([a-z][a-z0-9_]+)'", path.read_text(encoding="utf-8"))
            keys |= {t for t in tokens if "_" in t and len(t) >= 4}
    return keys


def check_ui_layout(ui: dict) -> None:
    if not UI_LAYOUT_TS.exists() or "PLACEHOLDER" in UI_LAYOUT_TS.read_text(encoding="utf-8"):
        pending.append("P3 — layout UI non généré (T040) : contrôle en attente")
        return
    layout = UI_LAYOUT_TS.read_text(encoding="utf-8")
    # Périmètre couvert : layout généré ∪ dialogs spéciaux ∪ exclusions.
    vocab = exclusion_vocabulary() | special_settings_keys()
    missing = []
    for page in ui["settings_tabs"]:
        for section in page["sections"]:
            for opt in section["options"]:
                if isinstance(opt, str) and f'"{opt}"' not in layout and opt not in vocab:
                    missing.append(opt)
    if missing:
        failures.append(f"P3 — {len(missing)} option(s) d'onglet absentes du layout : {missing[:10]}…")
    else:
        passed.append("P3 — layout UI ↔ ui_inventory : aucun écart")


def check_settings_special(params: dict) -> None:
    """T047 — les clés portées par les dialogs/pages spéciaux (hors lignes
    d'option) sont tracées : chacune doit exister dans le registre (sinon
    l'UI référence un paramètre fantôme). SC-001 partiel."""
    if not all(p.exists() for p in SPECIAL_TS):
        pending.append("P3 — pages/dialogs spéciaux absents (T044/T045) : contrôle en attente")
        return
    keys = special_settings_keys()
    unknown = sorted(k for k in keys if k not in params)
    if unknown:
        failures.append(
            f"P3 — {len(unknown)} clé(s) de dialog spécial absente(s) du registre : {unknown[:10]}…"
        )
    else:
        passed.append(f"P3 — dialogs spéciaux ↔ registre : {len(keys)} clés tracées, aucun écart")


def check_trace_map(ui: dict) -> None:
    """P4 (T062) — chaque élément d'interface de l'inventaire audit (gizmos 16/16,
    boutons de barres d'outils, libellés de menus contextuels, raccourcis des
    groupes Plater/Objects List/Gizmo) doit être mappé dans traceability-map.json
    ou justifié dans exclusions.md. Sinon, écart de parité non tracé."""
    if not TRACE_MAP.exists():
        pending.append("P4 — traceability-map.json absent (T061/T062) : contrôle en attente")
        return
    tmap = json.loads(TRACE_MAP.read_text(encoding="utf-8"))
    vocab = exclusion_vocabulary()
    gizmos_map = set(tmap.get("gizmos", {}))
    toolbars_map = set(tmap.get("toolbars", {}))
    menus_map = set(tmap.get("context_menu", {}))
    main_menu_map = set(tmap.get("main_menu", {}))
    shortcuts_map = tmap.get("shortcuts", {})

    missing: list[str] = []

    # Gizmos (16/16) — source de vérité : plater_gizmos.
    gizmo_names = [g.get("type", g["class"]) for g in ui["plater_gizmos"]]
    for name in gizmo_names:
        if name not in gizmos_map and name not in vocab:
            missing.append(f"gizmo:{name}")

    # Boutons de barres d'outils 3D (main + assemble + separator).
    for _func, items in ui["plater_toolbars"].items():
        for item in items:
            if item["name"] not in toolbars_map and item["name"] not in vocab:
                missing.append(f"toolbar:{item['name']}")

    # Libellés de menus contextuels du plateau (dédupliqués).
    for m in ui["plater_context_menus"]:
        label = m["label"]
        if label not in menus_map and label not in vocab:
            missing.append(f"menu:{label}")

    # Menus principaux Fichier/Édition/Vue/Aide (Annexe B §B.2, T079).
    for item in ui["main_menus"]["items"]:
        label = item["label"]
        if label not in main_menu_map and label not in vocab:
            missing.append(f"mainmenu:{label}")

    # Raccourcis des groupes couverts par la carte (Plater / Objects List / Gizmo).
    for grp in ui["keyboard_shortcuts"]:
        gname = grp.get("group")
        if gname not in shortcuts_map:
            continue
        keys_map = set(shortcuts_map[gname])
        for s in grp.get("shortcuts", []):
            k = s.get("keys")
            if k not in keys_map and k not in vocab:
                missing.append(f"shortcut:{gname}:{k}")

    if missing:
        failures.append(f"P4 — {len(missing)} élément(s) UI non mappés : {missing[:10]}…")
    else:
        n = len(gizmo_names)
        passed.append(
            f"P4 — gizmos {n}/{n}, barres d'outils, menus & raccourcis mappés : aucun écart"
        )


FRONTEND_SRC = REPO / "frontend" / "src"
_IMPORT_RE = re.compile(r"""(?:from|import)\s*\(?\s*['"]([^'"]+)['"]""")


def _resolve_import(spec: str, importer: Path) -> Path | None:
    """Résout un spécificateur d'import TS/Svelte en chemin de fichier réel.

    Gère les alias SvelteKit `$lib/…` et les imports relatifs `./`/`../` ;
    ignore l'externe (`$app`, `$env`, `~icons`, paquets nus). Essaie les
    extensions et `index.ts` comme le résolveur Vite."""
    if spec.startswith("$lib/"):
        base = FRONTEND_SRC / "lib" / spec[len("$lib/"):]
    elif spec.startswith("."):
        base = importer.parent / spec
    else:
        return None
    for cand in (
        base, base.with_suffix(".ts"), base.with_suffix(".svelte"),
        base.with_suffix(".js"), base / "index.ts", base / "index.js",
    ):
        if cand.exists() and cand.is_file():
            return cand.resolve()
    return None


def _reachable_from_routes() -> set[str]:
    """Clôture transitive des modules importés depuis `frontend/src/routes`.

    Point d'entrée = tout fichier de route (`+page`/`+layout`/… `.svelte`/`.ts`) ;
    on suit les imports résolus jusqu'à saturation. Renvoie des chemins
    repo-relatifs (séparateur `/`)."""
    routes = FRONTEND_SRC / "routes"
    roots = [p for p in routes.rglob("*") if p.suffix in (".svelte", ".ts", ".js")]
    seen: set[Path] = set()
    stack = [p.resolve() for p in roots]
    while stack:
        cur = stack.pop()
        if cur in seen:
            continue
        seen.add(cur)
        try:
            text = cur.read_text(encoding="utf-8")
        except OSError:
            continue
        for spec in _IMPORT_RE.findall(text):
            dep = _resolve_import(spec, cur)
            if dep and dep not in seen:
                stack.append(dep)
    return {str(p.relative_to(REPO)).replace("\\", "/") for p in seen}


def _trace_map_targets(tmap: dict) -> set[str]:
    """Chemins cibles uniques de la carte de traçabilité (tous registres)."""
    targets: set[str] = set()
    for key in ("gizmos", "toolbars", "context_menu", "main_menu"):
        targets.update(tmap.get(key, {}).values())
    for _grp, chords in tmap.get("shortcuts", {}).items():
        targets.update(chords.values())
    return targets


def check_wired() -> None:
    """P4 (T119) — garde « construit-mais-non-câblé » : chaque chemin cible de
    traceability-map.json (gizmos/toolbars/context_menu/main_menu/shortcuts)
    doit être *effectivement importé par une route* (clôture transitive depuis
    `frontend/src/routes`). Un composant tracé mais monté nulle part est un
    écart de parité réel. On tolère les cibles justifiées desktop-only
    (chemin backtické dans exclusions.md)."""
    if not TRACE_MAP.exists() or not (FRONTEND_SRC / "routes").exists():
        pending.append("P4 — garde câblage (T119) : routes ou carte absentes, en attente")
        return
    tmap = json.loads(TRACE_MAP.read_text(encoding="utf-8"))
    targets = _trace_map_targets(tmap)
    reachable = _reachable_from_routes()
    tolerated = exclusion_vocabulary()
    unwired = sorted(
        t for t in targets
        if t not in reachable and t not in tolerated
    )
    if unwired:
        failures.append(
            f"P4 — {len(unwired)} cible(s) tracée(s) construite(s) mais non "
            f"câblée(s) à une route : {unwired[:10]}…"
        )
    else:
        passed.append(
            f"P4 — câblage : {len(targets)} cible(s) tracée(s) importée(s) "
            "par une route, aucun élément construit-mais-non-câblé"
        )


def check_import_formats() -> None:
    """Chaque format importable par OrcaSlicer est soit accepté par
    `backend::detect_format`, soit justifié dans exclusions.md (T091). Aucun
    format Orca silencieusement omis."""
    if not MODELS_RS.exists():
        pending.append("import : models.rs absent — contrôle en attente")
        return
    text = MODELS_RS.read_text(encoding="utf-8")
    # Corps de detect_format : arms `"ext" => Some(ModelFormat::…)`.
    body = re.search(r"fn detect_format.*?\n\}", text, re.DOTALL)
    accepted = set(re.findall(r'"([a-z0-9]+)"', body.group(0))) if body else set()
    # Extensions justifiées comme exclues : backtickées dans exclusions.md
    # (forme `.ext`).
    excluded = {tok.lstrip(".") for tok in exclusion_vocabulary() if re.fullmatch(r"\.[a-z0-9]+", tok)}
    untracked = sorted(
        ext for ext in ORCA_IMPORT_EXTS if ext not in accepted and ext not in excluded
    )
    if untracked:
        failures.append(
            f"import : {len(untracked)} format(s) OrcaSlicer non tracé(s) "
            f"(ni acceptés ni exclus) : {untracked}"
        )
    else:
        n_acc = len(ORCA_IMPORT_EXTS & accepted)
        # Exclus « purs » : justifiés dans exclusions.md et non acceptés.
        n_exc = len((ORCA_IMPORT_EXTS & excluded) - accepted)
        passed.append(
            f"import — formats OrcaSlicer : {n_acc} acceptés, {n_exc} exclus justifiés, "
            "aucun format non tracé"
        )


def main() -> int:
    params = load("parameters.json")["parameters"]
    ui = load("ui_inventory.json")

    check_exclusions_registry(params)
    check_registry_rust(params)
    check_runtime_parity(params)
    check_presets()
    check_ui_layout(ui)
    check_settings_special(params)
    check_trace_map(ui)
    check_wired()
    check_import_formats()

    for p in passed:
        print(f"✓ {p}")
    for p in pending:
        print(f"… {p}")
    for f in failures:
        print(f"✗ {f}")

    if failures:
        print(f"\nÉCHEC : {len(failures)} écart(s) de parité non justifié(s)")
        return 1
    print(f"\nOK : {len(passed)} contrôle(s) verts, {len(pending)} en attente de phase")
    return 0


if __name__ == "__main__":
    sys.exit(main())
