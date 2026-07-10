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

if [ ! -f "$GEN_DIR/params.ts" ]; then
  cat > "$GEN_DIR/params.ts" << 'EOF'
// GÉNÉRÉ par scripts/codegen.sh — NE PAS ÉDITER À LA MAIN (constitution V).
// Registre des paramètres : étoffé par la tâche T040 (codegen complet
// depuis audit/parameters.json).
export const PARAMS_PLACEHOLDER = true;
EOF
fi

if [ ! -f "$GEN_DIR/ui-layout.ts" ]; then
  cat > "$GEN_DIR/ui-layout.ts" << 'EOF'
// GÉNÉRÉ par scripts/codegen.sh — NE PAS ÉDITER À LA MAIN (constitution V).
// Layout des onglets : étoffé par la tâche T040 (codegen complet
// depuis audit/ui_inventory.json).
export const UI_LAYOUT_PLACEHOLDER = true;
EOF
fi

echo "codegen OK — audit/, annexes/ et $GEN_DIR synchronisés"
