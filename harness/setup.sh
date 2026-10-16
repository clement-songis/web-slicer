#!/usr/bin/env bash
# Mise en place (une fois, puis après chaque mise à jour d'Orca ou de harness/oracle.json).
# À lancer dans `nix develop` (orca-slicer, bun, navigateurs Playwright).
set -euo pipefail
cd "$(dirname "$0")/.."
git submodule update --init --depth 1 vendor/OrcaSlicer
python3 tests/oracle/fixtures/gen_models.py
python3 harness/gen_features.py
(cd tests/oracle && bun install)
if [[ -n ${PLAYWRIGHT_BROWSERS_PATH:-} ]] && command -v playwright >/dev/null; then
	want=$(playwright --version | awk '{print $2}')
	(cd tests/oracle && bun add -d "@playwright/test@$want" >/dev/null) || true
fi
python3 tests/oracle/tools/make_refs.py --jobs "${JOBS:-$(nproc)}"
python3 harness/gen_features.py >/dev/null   # promeut needs_ref → red selon les références produites
python3 harness/next_feature.py >/dev/null || true
python3 harness/loopctl.py freeze
python3 harness/status.py
