#!/usr/bin/env python3
"""Génère / fusionne la liste des fonctionnalités à atteindre (parité Orca).

Sources : audit/parameters.json, audit/ui_inventory.json (extraits du code
d'OrcaSlicer). Produit :
  - $HARNESS_STATE/features.json (+ copie harness/features.json)
  - tests/oracle/params/cases.json  (un cas de test par paramètre)
  - tests/oracle/golden/cases.json  (corpus modèles × profils)

Idempotent : le statut, les tentatives et l'historique des features
existantes sont conservés. À lancer par un humain ou par la boucle, jamais
par l'agent (le résultat est gelé ensuite).
"""
from __future__ import annotations

import json
import re
import sys

sys.path.insert(0, str(__import__("pathlib").Path(__file__).resolve().parent))
from lib import AUDIT, HARNESS, ORACLE, load_features, locked, now, save_features  # noqa: E402

PARAMS = json.loads((AUDIT / "parameters.json").read_text())["parameters"]
UI = json.loads((AUDIT / "ui_inventory.json").read_text())
EXCL = json.loads((HARNESS / "exclusions.json").read_text())
ORACLE_CFG = json.loads((HARNESS / "oracle.json").read_text())


def slug(s: str) -> str:
    s = re.sub(r"&|%s|\?|…|\.\.\.", "", s)
    return re.sub(r"[^a-z0-9]+", "-", s.lower()).strip("-") or "x"


# ------------------------------------------------------------------ core

CORE = [
    ("core.harness-api", "API de test POST /api/_harness/slice (cf. CONTRACT.md §1) : "
     "slice un modèle avec une config Orca plate, G-code identique à orca-slicer CLI", []),
    ("core.auth-ui", "Inscription + connexion depuis l'UI (CONTRACT.md §2)", []),
    ("core.import", "Import STL depuis l'UI : l'objet apparaît dans la liste d'objets et la scène", ["core.auth-ui"]),
    ("core.settings-ui", "Dialogues de réglages process/filament/imprimante + attribut data-param sur chaque option", ["core.import"]),
    ("core.slice-ui", "Trancher depuis l'UI : progression puis aperçu avec couches", ["core.import"]),
    ("core.export-gcode", "Télécharger le G-code depuis l'UI ; identique (normalisé) à la réf. Orca pour le preset de base", ["core.slice-ui", "core.harness-api"]),
    ("core.preview", "Aperçu G-code : curseur de couches, légende par type de ligne", ["core.slice-ui"]),
    ("core.project-save", "Sauvegarder un projet, recharger la page : scène et réglages conservés", ["core.import"]),
    ("core.moonraker", "Déclarer une imprimante Moonraker, envoyer le G-code : fichier présent côté Moonraker", ["core.export-gcode"]),
]


# ------------------------------------------------------------------ params

TAB_SCOPE = (("TabPrinter", "printer"), ("TabFilament", "filament"), ("TabPrint", "print"))
MANUAL_KEYS = re.compile(
    r"^(post_process|inherits|compatible_.*|.*_settings_id|print_host.*|printhost.*|printer_model|"
    r"printer_variant|thumbnails.*|bed_custom_.*|filament_ids|default_.*_profile|host_type|"
    r"printer_technology|filament_vendor|preset_name|.*_notes|upward_compatible_machine|bed_exclude_area|"
    r"printable_area|extruder_offset|wipe_tower_x|wipe_tower_y|nozzle_diameter|extruder_colou?r|filament_colou?r)$"
)


def ui_index() -> dict[str, dict]:
    out: dict[str, dict] = {}
    # Ordre : dialogues de presets d'abord, puis réglages propres au plateau / à l'objet
    # (une clé n'est rattachée au plateau que si elle n'existe nulle part ailleurs, comme dans Orca).
    tabs = sorted(UI["settings_tabs"], key=lambda t: t.get("defined_in", "").startswith(("TabPrintModel", "TabPrintPlate")))
    for tab in tabs:
        where = tab.get("defined_in", "")
        if where.startswith("TabPrintModel"):
            scope = "object"
        elif where.startswith("TabPrintPlate"):
            scope = "plate"
        else:
            scope = next((s for p, s in TAB_SCOPE if where.startswith(p)), "print")
        title = "Extruder 1" if tab["title"] == "page_name" else tab["title"]
        for sec in tab["sections"]:
            for key in sec["options"]:
                if not isinstance(key, str):
                    continue  # option construite dynamiquement par Orca (non résolue par l'audit)
                k = key.split("#")[0]
                out.setdefault(k, {"tab": title, "section": sec["title"], "scope": scope,
                                   "tab_line": tab.get("line")})
    return out


def _num(x):
    try:
        return float(str(x).rstrip("%"))
    except (TypeError, ValueError):
        return None


def _fmt(x: float, as_int: bool) -> str:
    return str(int(round(x))) if as_int else (f"{x:.4f}".rstrip("0").rstrip(".") or "0")


def _pick_number(d: dict, as_int: bool) -> float:
    dflt = d.get("default")
    if isinstance(dflt, list):
        dflt = dflt[0] if dflt else None
    base = _num(dflt if dflt is not None else d.get("default_raw"))
    lo, hi = (v if isinstance(v, (int, float)) else None for v in (d.get("min"), d.get("max")))
    if base is None:
        base = 0.0
    cand = base + 1 if as_int else (base * 1.25 if base else 1.0)
    if hi is not None and cand > hi:
        cand = base - 1 if as_int else (base + lo) / 2 if lo is not None else base * 0.75
    if lo is not None and cand < lo:
        cand = lo + (1 if as_int else 0.1 * (abs(lo) or 1))
    if cand == base:
        cand = base + (1 if as_int else 0.5)
    return cand


def choose_value(key: str, d: dict):
    """Valeur de test (≠ défaut) au format JSON des profils Orca, ou None (cas manuel)."""
    t = d["type"]
    vec = t.endswith("s") or t.endswith("sNullable")
    base_t = re.sub(r"(s|sNullable)$", "", t) if vec else t
    if MANUAL_KEYS.match(key) or base_t in ("coPoint", "coPoints", "coPointsGroup") or t == "coPointsGroups":
        return None
    if base_t == "coBool":
        dflt = d.get("default")
        dflt = dflt[0] if isinstance(dflt, list) and dflt else dflt
        v = "0" if dflt in (True, 1, "1", "true") else "1"
    elif base_t == "coInt":
        v = _fmt(_pick_number(d, True), True)
    elif base_t == "coFloat":
        v = _fmt(_pick_number(d, False), False)
    elif base_t == "coPercent":
        v = _fmt(_pick_number(d, True), True) + "%"
    elif base_t == "coFloatOrPercent":
        raw = str(d.get("default_raw", ""))
        n = _pick_number(d, False)
        v = (_fmt(n, True) + "%") if raw.endswith("%") else _fmt(n, False)
    elif base_t == "coEnum":
        vals = d.get("enum_values") or next(
            (o["enum_values"] for o in PARAMS.values()
             if o.get("default_type") == d.get("default_type") and o.get("enum_values")), [])
        raw = str(d.get("default_raw", "")).lower()
        cands = [x for x in vals if x.lower() not in raw and slug(x) not in slug(raw)]
        if not cands:
            return None
        v = cands[0]
    elif base_t == "coString":
        if "gcode" in key or key.endswith("_template"):
            v = f"; HARNESS_MARK {key}"
        else:
            v = "harness_test"
    else:
        return None
    return [v] if vec else v


def param_features(feats: dict, cases: list) -> None:
    idx = ui_index()
    tier = {"simple": 20, "advanced": 30, "expert": 35, "develop": 40}
    for key, d in sorted(PARAMS.items()):
        grp = d["group"]
        fid = f"param.{key}"
        if grp == "sla" or grp.startswith(("cli:", "other:")):
            feats[fid] = {"kind": "param", "title": f"Paramètre {key}", "status": "excluded",
                          "reason": f"groupe {grp} : sans UI dans Orca (couvert par le moteur)"}
            continue
        ui = idx.get(key)
        value = choose_value(key, d)
        case = {"id": fid, "key": key, "type": d["type"], "value": value, "ui": ui is not None,
                "label": d.get("label"), "mode": d.get("mode", "advanced"),
                "nullable": d["type"].endswith("Nullable")}
        if ui:
            case.update(ui)
        cases.append(case)
        feats[fid] = {
            "kind": "param",
            "title": f"Paramètre {key} ({d.get('label') or '—'})" + ("" if ui else " — hors onglets"),
            "priority": tier.get(d.get("mode", "advanced"), 30) + (0 if ui else 5),
            "deps": ["core.harness-api"] + (["core.settings-ui"] if ui else []),
            "batch": f"{ui['scope']}/{ui['tab']}/{ui['section']}" if ui else f"hors-onglets/{d.get('category', 'divers')}",
            "test": "tests/oracle/params/params.spec.ts",
            "orca": [f"src/libslic3r/PrintConfig.cpp:{d.get('line')}"]
                    + ([f"src/slic3r/GUI/Tab.cpp:{ui['tab_line']} (onglet {ui['tab']} › {ui['section']})"] if ui else []),
            "default_status": "needs_ref" if value is not None else "needs_review",
            "reason": None if value is not None else "valeur de test à choisir à la main (harness/param_overrides.json)",
        }


# ------------------------------------------------------------------ golden

def golden_features(feats: dict, cases: list) -> None:
    for model in ORACLE_CFG["golden_models"]:
        for proc in ORACLE_CFG["golden_processes"]:
            fid = f"golden.{model}.{slug(proc)}"
            cases.append({"id": fid, "model": model, "process": proc})
            feats[fid] = {"kind": "golden", "title": f"Parité G-code : {model} × {proc}", "priority": 10,
                          "deps": ["core.harness-api"], "batch": "golden",
                          "test": "tests/oracle/golden/golden.spec.ts", "default_status": "needs_ref",
                          "orca": ["src/libslic3r/Print.cpp", "src/libslic3r/GCode.cpp"]}


# ------------------------------------------------------------------ UI (à écrire par l'auteur de tests)

GUI_FILE = {"MenuFactory": "src/slic3r/GUI/GUI_Factories.cpp", "MainFrame": "src/slic3r/GUI/MainFrame.cpp",
            "GLCanvas3D": "src/slic3r/GUI/GLCanvas3D.cpp", "Plater": "src/slic3r/GUI/Plater.cpp"}


def _gui_ref(defined_in: str | None, line) -> str:
    cls = (defined_in or "?").split("::")[0]
    return f"{GUI_FILE.get(cls, 'src/slic3r/GUI/' + cls + '.cpp')}:{line} ({defined_in})"


def ui_features(feats: dict) -> None:
    seen: dict[str, int] = {}

    def add(prefix, label, title, prio, deps, orca, batch):
        base = f"{prefix}.{slug(label)}"
        seen[base] = seen.get(base, 0) + 1
        fid = base if seen[base] == 1 else f"{base}-{seen[base]}"
        feats[fid] = {"kind": prefix, "title": title, "priority": prio, "deps": deps, "batch": batch,
                      "test": f"tests/oracle/scenarios/{fid}.spec.ts", "orca": orca,
                      "default_status": "needs_test"}

    base_deps = ["core.import"]
    add("ui", "object-settings", "Réglages par objet (liste d'objets › Ajouter des réglages) : surcharge d'un paramètre "
        "d'impression pour un seul objet, appliquée au G-code de cet objet uniquement", 45, ["core.settings-ui"],
        ["src/slic3r/GUI/GUI_ObjectSettings.cpp", "src/slic3r/GUI/GUI_ObjectList.cpp"], "ui")
    add("ui", "height-range-modifier", "Modificateur par plage de hauteur : réglages différents sur une plage de Z", 45,
        ["core.settings-ui"], ["src/slic3r/GUI/GUI_ObjectLayers.cpp"], "ui")
    add("ui", "preset-save-as", "Preset modifié (astérisque) puis « Enregistrer sous » un preset utilisateur, "
        "système resté intact, preset réutilisable dans un autre projet", 45, ["core.settings-ui"],
        ["src/slic3r/GUI/Tab.cpp", "src/slic3r/GUI/SavePresetDialog.cpp"], "ui")
    add("ui", "library-crud", "Bibliothèque (accueil « Récemment ouvert ») : renommer, dupliquer, supprimer un projet, "
        "persistés après rechargement", 45, ["core.project-save"], ["src/slic3r/GUI/WebViewDialog.cpp"], "ui")
    add("ui", "library-open", "Bibliothèque : « Ouvrir un projet » avec un modèle 3D crée un projet et ouvre l'éditeur",
        45, ["core.import"], ["src/slic3r/GUI/MainFrame.cpp"], "ui")
    add("ui", "queue", "File de tranchage : le tranchage lancé depuis l'éditeur apparaît dans l'historique, "
        "G-code téléchargeable", 45, ["core.slice-ui"], ["src/slic3r/GUI/BackgroundSlicingProcess.cpp"], "ui")
    for g in UI["plater_gizmos"]:
        name = g.get("type") or g["class"].replace("GLGizmo", "")
        add("gizmo", name, f"Gizmo {name} ({g['class']})", 50, base_deps,
            [f"src/slic3r/GUI/Gizmos/{g['class']}.cpp", f"src/slic3r/GUI/Gizmos/{g['class']}.hpp"], f"gizmo/{name}")
    for tb, items in UI["plater_toolbars"].items():
        for t in items:
            if "seperator" in t["name"] or "separator" in t["name"]:
                continue
            add("toolbar", t["name"], f"Barre d'outils : {t['name']} — {(t.get('tooltip') or '').splitlines()[0]}",
                50, base_deps, [f"src/slic3r/GUI/GLCanvas3D.cpp:{t['line']} ({tb})"], "toolbar")
    for m in UI["plater_context_menus"]:
        if m.get("kind") not in (None, "item", "check", "radio"):
            continue
        add("ctx", m["label"], f"Menu contextuel : {m['label']}", 60, base_deps,
            [_gui_ref(m.get("defined_in"), m.get("line"))], f"ctx/{m.get('menu_variable')}")
    for m in UI["main_menus"]["items"]:
        if m.get("kind") not in (None, "item", "check", "radio"):
            continue
        add("menu", m["label"], f"Menu {m.get('menu_variable')} : {m['label']} — {m.get('description') or ''}",
            65, base_deps, [_gui_ref(m.get("defined_in"), m.get("line"))], f"menu/{m.get('menu_variable')}")
    for grp in UI["keyboard_shortcuts"]:
        for s in grp["shortcuts"]:
            add("shortcut", s["keys"], f"Raccourci {s['keys']} : {s['action']} ({grp['group']})", 70,
                base_deps, ["src/slic3r/GUI/KBShortcutsDialog.cpp"], f"shortcut/{slug(grp['group'])}")


# Features « éditeur » ajoutées pendant la consolidation de parité : chaque id
# correspond à un tag @F: d'un test e2e existant (tests/oracle/e2e/*.spec.ts).
# (id, titre, fichier de test, dépendances, sources Orca)
EDITOR = [
    ("editor.object-settings", "Réglages par objet : wall_loops=5 sur un seul objet (document scène) atteint le G-code de cet objet",
     "editor-object-settings", ["core.project-save", "core.slice-ui"], ["src/slic3r/GUI/GUI_ObjectSettings.cpp"]),
    ("editor.object-panel", "Panneau des réglages par objet : clic droit › Add settings, recherche wall_loops=5, sauvegarde, G-code",
     "editor-object-panel", ["editor.object-settings"], ["src/slic3r/GUI/GUI_ObjectSettings.cpp", "src/slic3r/GUI/GUI_Factories.cpp"]),
    ("editor.paint-supports", "Peinture de supports : les facettes peintes « enforcer » génèrent des supports (mode manuel)",
     "editor-object-settings", ["core.project-save", "core.slice-ui"], ["src/slic3r/GUI/Gizmos/GLGizmoFdmSupports.cpp"]),
    ("editor.paint-supports-local", "Peinture de supports : supports seulement sous la zone peinte",
     "editor-object-settings", ["editor.paint-supports"], ["src/slic3r/GUI/Gizmos/GLGizmoFdmSupports.cpp"]),
    ("editor.paint-seam", "Peinture de couture : coutures placées sur la face peinte",
     "editor-object-settings", ["core.project-save", "core.slice-ui"], ["src/slic3r/GUI/Gizmos/GLGizmoSeam.cpp"]),
    ("editor.volume-negative", "Volume négatif ajouté depuis le menu contextuel, persistant, creuse l'objet au tranchage",
     "editor-object-settings", ["core.project-save", "core.slice-ui"], ["src/slic3r/GUI/GUI_ObjectList.cpp"]),
    ("editor.volume-modifier", "Modificateur : ses réglages s'appliquent dans son volume",
     "editor-object-settings", ["editor.volume-negative"], ["src/slic3r/GUI/GUI_ObjectList.cpp"]),
    ("editor.volume-blocker", "Bloqueur de supports : supprime les supports automatiques dans son volume",
     "editor-object-settings", ["editor.volume-negative"], ["src/slic3r/GUI/GUI_ObjectList.cpp"]),
    ("editor.volume-transform", "Volume sélectionné : panneau de transformation (position, rotation, échelle) persisté",
     "editor-volume-transform", ["editor.volume-modifier"], ["src/slic3r/GUI/GUI_ObjectManipulation.cpp"]),
    ("editor.export-3mf-paint-volumes", "Export 3MF : emporte peinture et volumes (types, réglages)",
     "editor-object-settings", ["editor.paint-supports", "editor.volume-modifier"], ["src/libslic3r/Format/bbs_3mf.cpp"]),
    ("editor.import-3mf-paint-volumes", "3MF : export → import → export restitue volumes et peinture",
     "editor-object-settings", ["editor.export-3mf-paint-volumes"], ["src/libslic3r/Format/bbs_3mf.cpp"]),
    ("editor.plate-settings", "Réglages par plateau (séquence d'impression, vase spirale) persistés et appliqués au G-code du plateau",
     "editor-plate-settings", ["core.project-save", "core.slice-ui"], ["src/slic3r/GUI/PlateSettingsDialog.cpp"]),
    ("editor.plate-duplicate", "Édition › Dupliquer le plateau courant : copie active avec ses objets",
     "editor-scene", ["core.import"], ["src/slic3r/GUI/PartPlate.cpp"]),
    ("editor.filament-slots", "Slots filament : ajout et retrait persistés après rechargement",
     "editor-scene", ["core.project-save"], ["src/slic3r/GUI/Plater.cpp"]),
    ("editor.preview-plates", "Aperçu multi-plateaux : curseur de couches, coloration par filament, export du plateau 2",
     "editor-preview", ["core.preview"], ["src/slic3r/GUI/GCodeViewer.cpp"]),
    ("editor.view-gcode-window", "Aperçu : fenêtre G-code synchronisée avec la couche affichée",
     "editor-view", ["core.preview"], ["src/slic3r/GUI/GCodeViewer.cpp"]),
    ("editor.export-toolpaths-obj", "Aperçu : export des trajets en OBJ",
     "editor-view", ["core.preview"], ["src/slic3r/GUI/GCodeViewer.cpp"]),
    ("editor.view-labels-overhang", "Vue : étiquettes d'objets et surplombs de la scène",
     "editor-view", ["core.import"], ["src/slic3r/GUI/GLCanvas3D.cpp"]),
    ("editor.undo-redo", "Édition › Annuler / Rétablir : suppression puis restauration de l'objet",
     "editor-history", ["core.import"], ["src/slic3r/GUI/Plater.cpp"]),
    ("editor.rename", "Objets › Renommer un objet et un plateau : persisté après rechargement",
     "editor-history", ["core.project-save"], ["src/slic3r/GUI/GUI_ObjectList.cpp"]),
    ("editor.menu-export-stl", "Menu Fichier › Export STL : un fichier binaire puis une archive par objet",
     "editor-menus", ["core.import"], ["src/slic3r/GUI/MainFrame.cpp"]),
    ("editor.menu-export-plates", "Menu Fichier › Exporter tous les plateaux : archive avec un G-code par plateau",
     "editor-menus", ["core.slice-ui"], ["src/slic3r/GUI/MainFrame.cpp"]),
    ("editor.menu-open-gcode", "Menu Fichier › Ouvrir un G-code local : aperçu sans tranchage",
     "editor-menus", ["core.preview"], ["src/slic3r/GUI/MainFrame.cpp"]),
    ("editor.menu-view-prefs", "Menu Vue : vue orthogonale et préférences",
     "editor-menus", ["core.import"], ["src/slic3r/GUI/MainFrame.cpp"]),
    ("editor.mesh-repair", "Outil maillage : réparation d'un maillage troué par le serveur, rechargé",
     "editor-mesh", ["core.import"], ["src/slic3r/Utils/FixModelByWin10.cpp"]),
    ("editor.mesh-simplify", "Outil maillage : simplification câblée à l'endpoint, maillage remplacé et persistant",
     "editor-mesh", ["core.import"], ["src/slic3r/GUI/Gizmos/GLGizmoSimplify.cpp"]),
    ("editor.mesh-simplify-error", "Outil maillage : erreur de simplification affichée clairement",
     "editor-mesh", ["editor.mesh-simplify"], ["src/slic3r/GUI/Gizmos/GLGizmoSimplify.cpp"]),
    ("editor.mesh-split", "Outil maillage : séparer en objets (deux cubes disjoints → deux objets)",
     "editor-split", ["core.import"], ["src/slic3r/GUI/GUI_ObjectList.cpp"]),
    ("editor.mesh-cut", "Outil maillage : coupe horizontale d'un cube de 20 mm",
     "editor-split", ["core.import"], ["src/slic3r/GUI/Gizmos/GLGizmoCut.cpp"]),
    ("editor.mesh-boolean", "Outil maillage : booléen différence A − B",
     "editor-split", ["core.import"], ["src/slic3r/GUI/Gizmos/GLGizmoMeshBoolean.cpp"]),
    ("editor.emboss-text", "Emboss : texte en relief créé et tranché",
     "editor-emboss", ["core.import", "core.slice-ui"], ["src/slic3r/GUI/Gizmos/GLGizmoEmboss.cpp"]),
    ("editor.calibration-temp", "Calibration › Température : tour tranchée, M104 S230…S190 par paliers",
     "editor-calibration", ["core.slice-ui"], ["src/slic3r/GUI/calib_dlg.cpp", "src/libslic3r/calib.cpp"]),
    ("editor.calibration-retraction", "Calibration › Rétraction : longueur croissante par paliers",
     "editor-calibration-engine", ["core.slice-ui"], ["src/slic3r/GUI/calib_dlg.cpp", "src/libslic3r/calib.cpp"]),
    ("editor.calib-pa-pattern", "Calibration › Pressure advance pattern : motif injecté avec cinq paliers de PA",
     "editor-calibration-shaping", ["core.slice-ui"], ["src/slic3r/GUI/Plater.cpp", "src/libslic3r/calib.cpp"]),
    ("editor.calib-input-shaping", "Calibration › Input shaping : fréquences variables couche par couche",
     "editor-calibration-shaping", ["core.slice-ui"], ["src/slic3r/GUI/calib_dlg.cpp", "src/libslic3r/calib.cpp"]),
    ("editor.calib-cornering", "Calibration › Virages : jerk ou junction deviation variable par couche",
     "editor-calibration-shaping", ["core.slice-ui"], ["src/slic3r/GUI/calib_dlg.cpp", "src/libslic3r/calib.cpp"]),
    ("editor.volume-gizmo", "Volume sélectionné : gizmo 3D, déplacement persisté et un seul pas d'historique par geste",
     "editor-volume-gizmo", ["editor.volume-transform"], ["src/slic3r/GUI/GUI_ObjectManipulation.cpp", "src/slic3r/GUI/Gizmos/GLGizmoMove.cpp"]),
    ("ui.import-3mf-multi", "Import d'un 3MF multi-objets (produit par Orca) dans un projet : un objet nommé par "
     "objet du 3MF (transformations, volumes, réglages), persistant après rechargement",
     "import-3mf-multi", ["core.import"], ["src/slic3r/GUI/Plater.cpp", "src/libslic3r/Format/bbs_3mf.cpp"]),
    ("ui.device", "Onglet Appareil (Moonraker) : consignes de température, déplacements/origine, ventilateur, "
     "fichiers de l'imprimante et lancement, couche/progression/durée, webcam",
     "device", ["core.moonraker"], ["src/slic3r/GUI/DeviceManager.cpp", "src/slic3r/GUI/StatusPanel.cpp"]),
    ("preview.engine", "Aperçu alimenté par GCodeProcessor (moteur) : méta au tranchage, mouvements par fenêtre de "
     "couches, Résumé (temps simulé = en-tête du G-code), vitesse / débit réels, temps de couche simulés",
     "engine-preview", ["core.preview"], ["src/libslic3r/GCode/GCodeProcessor.cpp", "src/slic3r/GUI/GCodeViewer.cpp"]),
]


def editor_features(feats: dict) -> None:
    for i, (fid, title, spec, deps, orca) in enumerate(EDITOR):
        feats[fid] = {"kind": "editor", "title": title, "priority": 40 + i / 100, "deps": deps, "batch": None,
                      "test": f"tests/oracle/e2e/{spec}.spec.ts", "orca": orca, "default_status": "red"}


# ------------------------------------------------------------------ fusion

def excluded_reason(fid: str) -> str | None:
    if fid in EXCL.get("ids", {}):
        return EXCL["ids"][fid]
    for p in EXCL.get("patterns", []):
        if re.search(p["re"], fid):
            return p["reason"]
    return None


def main() -> None:
    fresh: dict[str, dict] = {}
    for i, (fid, title, deps) in enumerate(CORE):
        fresh[fid] = {"kind": "core", "title": title, "priority": i, "deps": deps, "batch": None,
                      "test": "tests/oracle/e2e/core.spec.ts", "orca": [], "default_status": "red"}
    param_cases: list = []
    golden_cases: list = []
    golden_features(fresh, golden_cases)
    param_features(fresh, param_cases)
    ui_features(fresh)
    editor_features(fresh)

    # Trois scénarios e2e intégrés après preuve navigateur : rattache chaque
    # feature portant leur tag @F: au fichier réel au lieu d'un scénario
    # unitaire inexistant. Restreint à ces fichiers pour ne pas recâbler les
    # autres lots pendant leur intégration.
    for spec_name in ("editor-volume-gizmo", "editor-calibration-shaping", "editor-shortcuts-ctx",
                      "menu-a", "menu-a-drc", "menu-b", "ctx-menus", "ctx-menus-2", "shortcuts-rest",
                      "ui-gizmo-toolbar"):
        spec = ORACLE / "e2e" / f"{spec_name}.spec.ts"
        for fid in set(re.findall(r"@F:([\w.-]+)", spec.read_text())):
            if fid in fresh:
                fresh[fid]["test"] = f"tests/oracle/e2e/{spec_name}.spec.ts"

    req_dir = ORACLE / "params" / "requires"
    for c in param_cases:
        f = req_dir / f"{c['key']}.json"
        if f.exists():
            c["requires"] = json.loads(f.read_text()).get("requires", {})
    overrides = json.loads((HARNESS / "param_overrides.json").read_text())
    for c in param_cases:
        if c["key"] in overrides:
            o = overrides[c["key"]]
            if isinstance(o, dict):
                c["value"] = o.get("value")
                if o.get("model"):
                    c["model"] = o["model"]
                if o.get("requires"):
                    c["requires"] = o["requires"]
            else:
                c["value"] = o
            fresh[c["id"]]["default_status"] = "needs_ref"
            fresh[c["id"]]["reason"] = None

    with locked():
        data = load_features()
        old = data["features"]
        merged = {}
        for fid, f in fresh.items():
            prev = old.get(fid, {})
            default = f.pop("default_status", "red")
            reason = excluded_reason(fid)
            if reason:
                status = "excluded"
                f["reason"] = reason
            elif f.get("status") == "excluded":
                status = "excluded"
            elif prev.get("status") and prev["status"] != "excluded":
                status = prev["status"]
            else:
                status = default
            merged[fid] = {**f, "status": status,
                           "attempts": prev.get("attempts", 0),
                           "last_error": prev.get("last_error"),
                           "last_note": prev.get("last_note"),
                           "history": prev.get("history", [])}
            if merged[fid].get("reason") is None:
                merged[fid].pop("reason", None)
        data["features"] = merged
        data["generated_at"] = now()
        save_features(data)

    (ORACLE / "params").mkdir(parents=True, exist_ok=True)
    (ORACLE / "golden").mkdir(parents=True, exist_ok=True)
    (ORACLE / "params" / "cases.json").write_text(json.dumps(param_cases, indent=1, ensure_ascii=False) + "\n")
    (ORACLE / "golden" / "cases.json").write_text(json.dumps(golden_cases, indent=1, ensure_ascii=False) + "\n")

    from collections import Counter
    c = Counter((f["kind"], f["status"]) for f in merged.values())
    print(f"{len(merged)} features")
    for (k, s), n in sorted(c.items()):
        print(f"  {k:9} {s:13} {n}")


if __name__ == "__main__":
    main()
