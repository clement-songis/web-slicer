#!/usr/bin/env bash
# Boucle autonome : une session Claude courte par lot, jugée par verify.py.
#
#   harness/loop.sh                 jusqu'à épuisement du budget ou du travail
#   HARNESS_MAX_USD=30 HARNESS_MAX_SESSIONS=80 harness/loop.sh
#
# Variables : HARNESS_MAX_USD (25) · HARNESS_MAX_SESSIONS (60) · HARNESS_MAX_TURNS (80)
#             HARNESS_MODEL_IMPL (sonnet) · HARNESS_MODEL_TESTS (opus) · HARNESS_MODEL_REVIEW (opus)
#             HARNESS_CLAUDE_FLAGS (défaut : --permission-mode acceptEdits ; en conteneur :
#             --dangerously-skip-permissions) · HARNESS_NIGHTLY=1 → régression complète à la fin
set -uo pipefail
cd "$(dirname "$0")/.."
H=harness
MAX_USD=${HARNESS_MAX_USD:-25}
MAX_SESSIONS=${HARNESS_MAX_SESSIONS:-60}
MAX_TURNS=${HARNESS_MAX_TURNS:-80}
read -r -a FLAGS <<<"${HARNESS_CLAUDE_FLAGS:---permission-mode acceptEdits}"
START=$(date +%Y-%m-%dT%H:%M:%S)
mkdir -p $H/logs

python3 $H/loopctl.py preflight || exit 1

session=0
while ((session < MAX_SESSIONS)); do
	spent=$(python3 $H/loopctl.py spent "$START")
	if python3 -c "import sys; sys.exit(0 if float('$spent') >= float('$MAX_USD') else 1)"; then
		echo "Budget atteint (\$$spent / \$$MAX_USD)."
		break
	fi
	session=$((session + 1))
	task=$(python3 $H/next_feature.py --session "$session")
	rc=$?
	if ((rc == 3)); then
		echo "Plus rien d'exécutable : $(echo "$task" | python3 -c 'import json,sys;print(json.load(sys.stdin)["summary"])')"
		break
	elif ((rc != 0)); then
		echo "next_feature a échoué"; break
	fi
	mode=$(echo "$task" | python3 -c 'import json,sys;print(json.load(sys.stdin)["mode"])')
	ids=$(echo "$task" | python3 -c 'import json,sys;print(" ".join(json.load(sys.stdin)["ids"]))')
	model=$(echo "$task" | python3 -c 'import json,sys;print(json.load(sys.stdin)["model"])')
	context=$(echo "$task" | python3 -c 'import json,sys;print(json.load(sys.stdin)["context"])')
	base=$(git rev-parse HEAD)
	out=$H/logs/session-$(date +%Y%m%d-%H%M%S)-$session.json
	echo "[$session] $mode ($model) : $ids   (dépensé \$$spent)"

	HARNESS_MODE=$mode HARNESS_FEATURE=$ids claude -p "$(cat "$H/PROMPT_$mode.md")

$context" --model "$model" --max-turns "$MAX_TURNS" --output-format json "${FLAGS[@]}" >"$out" 2>"$out.err"

	if [[ $mode == tests ]]; then
		if [[ $ids == param.* ]]; then
			vrc=0   # dépendances de paramètres : jugées après régénération des références Orca
		else
			python3 $H/verify.py --mode tests $ids
			vrc=$?
		fi
		for id in $ids; do
			for f in tests/oracle/scenarios/$id.spec.ts tests/oracle/params/requires/${id#param.}.json; do
				[[ -f $f ]] && git add "$f"
			done
		done
		python3 $H/loopctl.py requires "$ids"
		git commit -qm "test(oracle): scenarios for $ids" -m "Co-Authored-By: Claude <noreply@anthropic.com>" 2>/dev/null
		git checkout -q -- . && git clean -fdq -- frontend backend engine
		python3 $H/loopctl.py freeze >/dev/null
	else
		python3 $H/verify.py --mode "$mode" $ids
		vrc=$?
		if ((vrc == 2 || vrc == 4)); then
			echo "  ↺ session annulée (régression ou altération des tests) : retour à ${base:0:8}"
			git reset -q --hard "$base"
		elif [[ -n $(git status --porcelain -- frontend backend engine Cargo.toml Cargo.lock) ]]; then
			git add -A -- frontend backend engine Cargo.toml Cargo.lock && git commit -qm "wip: partial work on $ids" -m "Co-Authored-By: Claude <noreply@anthropic.com>"
		fi
	fi
	python3 $H/loopctl.py after "$mode" "$ids" "$out" "$vrc" "$session" "$model"
done

if [[ ${HARNESS_NIGHTLY:-0} == 1 ]]; then
	echo "Régression complète…"
	python3 $H/verify.py --all
fi
python3 $H/status.py
