#!/usr/bin/env bash
# Pile de test isolée : backend (HARNESS_API=1, base dédiée), front Vite, Moonraker virtuel (Docker, optionnel ;
# sans Docker : harness/moonraker-mock.mjs, sous-ensemble Node du contrat R10 ; HARNESS_MOONRAKER=0 pour aucun).
#   stack.sh up      démarre ou rafraîchit (rebuild backend si le code Rust a changé), puis préchauffe
#   stack.sh warmup  relance seulement le préchauffage navigateur (pile déjà démarrée)
#   stack.sh down    arrête tout
#   stack.sh logs    dernières lignes des journaux
# À lancer depuis `nix develop` (toolchain Rust, libslic3r, orca-slicer, navigateurs Playwright).
# L'environnement de l'appelant (dont DATABASE_URL appliqué dans le dev shell) est transmis tel quel au backend.
#
# Front : servi par harness/vite-e2e.mjs (config du front + server.ws=false/hmr=false) pour qu'aucun
# rechargement Vite ne puisse naviguer une page pendant un test ; les processus orphelins qui
# occupent les ports de la pile sont repris (sinon wait_http validerait un ancien serveur).
# Préchauffage : harness/warmup.mjs (Chromium réel) — HARNESS_WARMUP=0 pour le sauter,
# HARNESS_WARMUP_TIMEOUT (s, défaut 300), HARNESS_WARMUP_PASSES (défaut 5).
set -euo pipefail
cd "$(dirname "$0")/.."
REPO=$PWD
STATE=${HARNESS_STATE:-$HOME/.local/state/web-slicer-harness}
RUN=$STATE/stack
mkdir -p "$RUN"
BACK_PORT=${HARNESS_BACKEND_PORT:-18080}
FRONT_PORT=${HARNESS_FRONTEND_PORT:-15173}
MOON_PORT=${HARNESS_MOONRAKER_PORT:-17125}

alive() { [[ -f $RUN/$1.pid ]] && kill -0 "$(cat "$RUN/$1.pid")" 2>/dev/null; }
wait_dead() { # pid timeout_s
	for _ in $(seq $(($2 * 4))); do kill -0 "$1" 2>/dev/null || return 0; sleep 0.25; done
	kill -9 "$1" 2>/dev/null || true; sleep 0.5
}
stop() {
	if alive "$1"; then
		local pid; pid=$(cat "$RUN/$1.pid")
		# Le front est lancé avec setsid : son groupe ne contient que lui et ses enfants.
		if [[ $(ps -o pgid= -p "$pid" 2>/dev/null | tr -d ' ') == "$pid" ]]; then kill -- "-$pid" 2>/dev/null || true
		else kill "$pid" 2>/dev/null || true; fi
		wait_dead "$pid" 10
	fi
	rm -f "$RUN/$1.pid"
}
wait_http() { # url timeout_s
	for _ in $(seq "$2"); do curl -fsS -o /dev/null "$1" 2>/dev/null && return 0; sleep 1; done
	return 1
}
port_pids() { ss -ltnpH "sport = :$1" 2>/dev/null | grep -o 'pid=[0-9]*' | cut -d= -f2 | sort -u || true; }
# Libère un port occupé par un ancien processus de cette pile (orphelin d'un précédent `up`) ;
# refuse de tuer un processus étranger.
reclaim_port() { # port pattern label
	local pid cmd cwd
	for pid in $(port_pids "$1"); do
		cmd=$(tr '\0' ' ' <"/proc/$pid/cmdline" 2>/dev/null || true)
		cwd=$(readlink "/proc/$pid/cwd" 2>/dev/null || true)
		if [[ $cmd == *"$2"* || $cwd == "$REPO" || $cwd == "$REPO"/* ]]; then
			echo "[stack] $3 : port $1 tenu par un ancien processus (pid $pid : ${cmd:0:120}) → arrêt"
			kill "$pid" 2>/dev/null || true
			wait_dead "$pid" 10
		else
			echo "[stack] $3 : port $1 occupé par un processus étranger (pid $pid : ${cmd:0:120})"
			exit 1
		fi
	done
	if [[ -n $(port_pids "$1") ]]; then echo "[stack] $3 : port $1 toujours occupé"; exit 1; fi
}
owns_port() { [[ -f $RUN/$1.pid ]] && port_pids "$2" | grep -qx "$(cat "$RUN/$1.pid")"; }
src_hash() { find backend engine Cargo.toml Cargo.lock -type f \( -name '*.rs' -o -name '*.toml' -o -name '*.lock' -o -name '*.cpp' -o -name '*.h' \) -newer "$RUN/backend.built" 2>/dev/null | head -1; }

up_backend() {
	if [[ ! -f $RUN/backend.built ]] || [[ -n $(src_hash) ]] || ! alive backend || ! owns_port backend "$BACK_PORT"; then
		echo "[stack] build backend…"
		cargo build --workspace --bins ${HARNESS_CARGO_FLAGS:-} >"$RUN/build.log" 2>&1 || { tail -60 "$RUN/build.log"; exit 1; }
		touch "$RUN/backend.built"
		stop backend
		local bin; bin=$(cargo metadata --format-version 1 --no-deps | python3 -c 'import json,sys;print(json.load(sys.stdin)["target_directory"])')/debug
		reclaim_port "$BACK_PORT" "$bin/backend" backend
		echo "[stack] backend : DATABASE_URL=${DATABASE_URL:-(non défini : $STATE/data/web-slicer.db)}"
		DATA_DIR=$STATE/data BIND=127.0.0.1:$BACK_PORT HARNESS_API=1 \
			INSTANCE_SECRET_KEY=${INSTANCE_SECRET_KEY:-harness-test-secret-key-0123456789abcdef} \
			RUST_LOG=${RUST_LOG:-warn} nohup "$bin/backend" >"$RUN/backend.log" 2>&1 &
		echo $! >"$RUN/backend.pid"
		wait_http "http://127.0.0.1:$BACK_PORT/api/health" 300 || { echo "[stack] backend ne répond pas"; tail -60 "$RUN/backend.log"; exit 1; }
		owns_port backend "$BACK_PORT" || { echo "[stack] le port $BACK_PORT n'est pas tenu par le backend lancé"; tail -60 "$RUN/backend.log"; exit 1; }
	fi
}

front_mode() { echo "vite-e2e:ws-off:$(cksum <harness/vite-e2e.mjs | cut -d' ' -f1):$BACK_PORT"; }

up_frontend() {
	local mode; mode=$(front_mode)
	if alive frontend && owns_port frontend "$FRONT_PORT" && [[ $(cat "$RUN/frontend.mode" 2>/dev/null) == "$mode" ]]; then
		echo "[stack] front déjà lancé (pid $(cat "$RUN/frontend.pid"), $mode)"
	else
		echo "[stack] front (vite dev, ws/hmr off)…"
		stop frontend
		reclaim_port "$FRONT_PORT" "$REPO/frontend/node_modules" front
		rm -f "$RUN/frontend.mode"
		[[ -d frontend/node_modules ]] || (cd frontend && bun install --frozen-lockfile >"$RUN/bun.log" 2>&1)
		(cd frontend && node scripts/copy-printer-covers.mjs) >"$RUN/frontend.log" 2>&1 || { tail -30 "$RUN/frontend.log"; exit 1; }
		# vite-e2e.mjs écrit lui-même son PID (chef de session setsid) une fois à l'écoute.
		(cd frontend && BACKEND_URL=http://127.0.0.1:$BACK_PORT HARNESS_E2E=1 HARNESS_FRONTEND_PORT=$FRONT_PORT \
			HARNESS_FRONTEND_PIDFILE=$RUN/frontend.pid setsid nohup node "$REPO/harness/vite-e2e.mjs" >>"$RUN/frontend.log" 2>&1 &)
		for _ in $(seq 240); do
			[[ -s $RUN/frontend.pid ]] && break
			grep -aq 'error when starting\|Error \[ERR_\|^Error:' "$RUN/frontend.log" && { echo "[stack] le front n'a pas démarré"; tail -40 "$RUN/frontend.log"; exit 1; }
			sleep 1
		done
		echo "$mode" >"$RUN/frontend.mode"
	fi
	wait_http "http://127.0.0.1:$FRONT_PORT/" 240 || { echo "[stack] front ne répond pas"; tail -60 "$RUN/frontend.log"; exit 1; }
	owns_port frontend "$FRONT_PORT" || { echo "[stack] le port $FRONT_PORT n'est pas tenu par le front lancé"; tail -60 "$RUN/frontend.log"; exit 1; }
}

# Repli sans Docker : Moonraker simulé en Node (pid suivi, arrêté par `down`).
up_moonraker_mock() {
	local url=http://127.0.0.1:$MOON_PORT
	if ! alive moonraker-mock; then
		# Port déjà servi par un Moonraker (mock lancé à la main…) : on le réutilise.
		if ! curl -fsS -m 2 "$url/server/info" >/dev/null 2>&1; then
			nohup node harness/moonraker-mock.mjs "$MOON_PORT" >"$RUN/moonraker-mock.log" 2>&1 &
			echo $! >"$RUN/moonraker-mock.pid"
		fi
	fi
	if wait_http "$url/server/info" 30; then
		echo "[stack] docker absent : Moonraker simulé sur $url"
		echo "MOONRAKER_URL=$url" >>"$STATE/stack.env"
	else
		echo "[stack] Moonraker simulé injoignable : core.moonraker sera ignoré"
		tail -20 "$RUN/moonraker-mock.log" 2>/dev/null || true
	fi
}

up_moonraker() {
	[[ ${HARNESS_MOONRAKER:-1} == 0 ]] && return 0
	command -v docker >/dev/null || { up_moonraker_mock; return 0; }
	if ! docker ps --format '{{.Names}}' | grep -qx ws-harness-moonraker; then
		docker rm -f ws-harness-moonraker >/dev/null 2>&1 || true
		docker run -d --name ws-harness-moonraker -p "127.0.0.1:$MOON_PORT:7125" \
			"${HARNESS_MOONRAKER_IMAGE:-ghcr.io/mainsail-crew/virtual-klipper-printer:master}" >/dev/null \
			|| { echo "[stack] impossible de lancer le Moonraker virtuel"; return 0; }
	fi
	wait_http "http://127.0.0.1:$MOON_PORT/server/info" 120 && echo "MOONRAKER_URL=http://127.0.0.1:$MOON_PORT" >>"$STATE/stack.env"
}

warmup() {
	[[ ${HARNESS_WARMUP:-1} == 0 ]] && { echo "[stack] préchauffage ignoré (HARNESS_WARMUP=0)"; return 0; }
	local t=${HARNESS_WARMUP_TIMEOUT:-300}
	{ alive frontend && owns_port frontend "$FRONT_PORT"; } || { echo "[stack] préchauffage : front non démarré (lancer stack.sh up)"; exit 1; }
	exec 9>"$RUN/warmup.lock"
	flock -w "$t" 9 || { echo "[stack] préchauffage : verrou toujours tenu après ${t}s"; exit 1; }
	echo "[stack] préchauffage navigateur (register, login, setup, projet)…"
	if ! BASE_URL=http://127.0.0.1:$FRONT_PORT HARNESS_FRONTEND_LOG=$RUN/frontend.log HARNESS_WARMUP_TIMEOUT=$t \
		timeout --kill-after=10 $((t + 30)) node harness/warmup.mjs 2>&1 | tee "$RUN/warmup.log"; then
		echo "[stack] préchauffage échoué : pile instable (détails : $RUN/warmup.log)"
		echo "== frontend.log"; tail -30 "$RUN/frontend.log" | tr -d '\000'
		exit 1
	fi
	flock -u 9
}

case ${1:-up} in
up)
	: >"$STATE/stack.env"
	up_backend
	up_frontend
	echo "BASE_URL=http://127.0.0.1:$FRONT_PORT" >>"$STATE/stack.env"
	up_moonraker
	[[ -d tests/oracle/node_modules ]] || (cd tests/oracle && bun install >"$RUN/oracle-install.log" 2>&1)
	warmup
	echo "[stack] prête"
	;;
warmup)
	warmup
	;;
down)
	stop frontend; stop backend; stop moonraker-mock
	command -v docker >/dev/null && { docker rm -f ws-harness-moonraker >/dev/null 2>&1 || true; }
	;;
logs)
	for f in build backend frontend warmup moonraker-mock; do echo "== $f"; tail -40 "$RUN/$f.log" 2>/dev/null | tr -d '\000' || true; done
	;;
*) echo "usage: $0 up|warmup|down|logs"; exit 2 ;;
esac
