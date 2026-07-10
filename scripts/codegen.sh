#!/usr/bin/env bash
# Régénère tous les artefacts de parité depuis vendor/OrcaSlicer.
# Idempotent : deux exécutions successives produisent des sorties identiques.
#
# Chaîne (R2/R3, plan.md) :
#   1. audit/run_all.py              → audit/*.json (sources de vérité)
#   2. audit/generate_parity_annexes → annexes normatives de la spec
#   3. génération frontend/src/generated/ (params.ts, ui-layout.ts)
#      — étoffée par T040 ; le registre Rust : audit/generate_registry_rs.py → engine/src/params/registry.rs (committé).
set -euo pipefail
cd "$(dirname "$0")/.."

python3 audit/run_all.py
python3 audit/generate_parity_annexes.py
python3 audit/generate_registry_rs.py
python3 audit/generate_legacy_rs.py
# les fichiers générés doivent rester rustfmt-stables (gate cargo fmt --check)
cargo fmt -p engine 2>/dev/null || true

GEN_DIR=frontend/src/generated
mkdir -p "$GEN_DIR"

# Registre params + layout UI (T040) depuis audit/parameters.json et
# audit/ui_inventory.json — sortie déterministe, fraîcheur testée en vitest.
python3 audit/generate_frontend_ts.py

echo "codegen OK — audit/, annexes/ et $GEN_DIR synchronisés"
