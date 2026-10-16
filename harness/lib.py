"""Utilitaires partagés du harness (chemins, état autoritaire, verrous).

L'état autoritaire (features + empreintes des tests gelés) vit HORS du dépôt,
dans $HARNESS_STATE (défaut ~/.local/state/web-slicer-harness) : l'agent n'y
a pas accès (hook guard.py). `harness/features.json` dans le dépôt n'est
qu'une copie de lecture, réécrite par la boucle.
"""
from __future__ import annotations

import fcntl
import hashlib
import json
import os
import tarfile
import time
from contextlib import contextmanager
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
HARNESS = REPO / "harness"
ORACLE = REPO / "tests" / "oracle"
AUDIT = REPO / "audit"
VENDOR = REPO / "vendor" / "OrcaSlicer"
STATE = Path(os.environ.get("HARNESS_STATE", Path.home() / ".local/state/web-slicer-harness"))
FEATURES_STATE = STATE / "features.json"
FEATURES_MIRROR = HARNESS / "features.json"
FROZEN_MANIFEST = STATE / "frozen.json"
FROZEN_SNAPSHOT = STATE / "frozen.tar"
LOGS = HARNESS / "logs"

MAX_ATTEMPTS = int(os.environ.get("HARNESS_MAX_ATTEMPTS", "3"))

# Chemins gelés : l'agent d'implémentation ne doit jamais les modifier.
FROZEN_ROOTS = ["tests/oracle", "harness", ".claude/settings.json"]
FROZEN_IGNORE = (
    "tests/oracle/node_modules",
    "tests/oracle/test-results",
    "tests/oracle/playwright-report",
    "tests/oracle/.cache",
    "harness/logs",
    "harness/features.json",
    "harness/__pycache__",
)

STATUSES = (
    "needs_ref",    # en attente des références Orca (make_golden.sh)
    "needs_test",   # test à écrire par une session « auteur de tests »
    "red",          # test gelé, échoue : prêt à implémenter
    "passing",      # validé par verify.py
    "blocked",      # MAX_ATTEMPTS échecs : revue Opus / humaine
    "needs_review", # cas douteux (réf. Orca absente, test vert dès sa création…)
    "excluded",     # exclusion décidée par un humain (harness/exclusions.json)
)


def ensure_state() -> None:
    STATE.mkdir(parents=True, exist_ok=True)
    LOGS.mkdir(parents=True, exist_ok=True)


@contextmanager
def locked():
    ensure_state()
    with open(STATE / ".lock", "w") as fh:
        fcntl.flock(fh, fcntl.LOCK_EX)
        try:
            yield
        finally:
            fcntl.flock(fh, fcntl.LOCK_UN)


def load_features() -> dict:
    if FEATURES_STATE.exists():
        return json.loads(FEATURES_STATE.read_text())
    return {"version": 1, "features": {}}


def save_features(data: dict) -> None:
    ensure_state()
    text = json.dumps(data, indent=1, ensure_ascii=False, sort_keys=False) + "\n"
    tmp = FEATURES_STATE.with_suffix(".tmp")
    tmp.write_text(text)
    tmp.replace(FEATURES_STATE)
    FEATURES_MIRROR.write_text(text)


def now() -> str:
    return time.strftime("%Y-%m-%dT%H:%M:%S")


# ---------------------------------------------------------------- gel des tests

def _frozen_files() -> list[Path]:
    out: list[Path] = []
    for root in FROZEN_ROOTS:
        p = REPO / root
        if p.is_file():
            out.append(p)
            continue
        for f in sorted(p.rglob("*")):
            rel = f.relative_to(REPO).as_posix()
            if f.is_file() and not rel.startswith(FROZEN_IGNORE) and "__pycache__" not in rel:
                out.append(f)
    return out


def _digest(p: Path) -> str:
    return hashlib.sha256(p.read_bytes()).hexdigest()


def freeze() -> int:
    """Enregistre l'état courant des chemins gelés (manifeste + archive)."""
    ensure_state()
    files = _frozen_files()
    manifest = {f.relative_to(REPO).as_posix(): _digest(f) for f in files}
    FROZEN_MANIFEST.write_text(json.dumps(manifest, indent=0))
    with tarfile.open(FROZEN_SNAPSHOT, "w") as tar:
        for f in files:
            tar.add(f, arcname=f.relative_to(REPO).as_posix())
    return len(manifest)


def tamper_report() -> list[str]:
    """Liste des fichiers gelés modifiés / ajoutés / supprimés depuis le gel."""
    if not FROZEN_MANIFEST.exists():
        return []
    manifest = json.loads(FROZEN_MANIFEST.read_text())
    current = {f.relative_to(REPO).as_posix(): _digest(f) for f in _frozen_files()}
    diffs = [f"modifié: {k}" for k, v in manifest.items() if k in current and current[k] != v]
    diffs += [f"supprimé: {k}" for k in manifest if k not in current]
    diffs += [f"ajouté: {k}" for k in current if k not in manifest]
    return diffs


def restore_frozen() -> None:
    """Restaure les chemins gelés depuis l'archive (annule toute altération)."""
    if not FROZEN_SNAPSHOT.exists():
        return
    manifest = json.loads(FROZEN_MANIFEST.read_text())
    for rel in {f.relative_to(REPO).as_posix() for f in _frozen_files()} - set(manifest):
        (REPO / rel).unlink(missing_ok=True)
    with tarfile.open(FROZEN_SNAPSHOT) as tar:
        tar.extractall(REPO, filter="data")
