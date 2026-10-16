#!/usr/bin/env bash
# Pile de test isolée : backend (HARNESS_API=1, base dédiée), front Vite, Moonraker virtuel (Docker, optionnel).
#   stack.sh up     démarre ou rafraîchit (rebuild backend si le code Rust a changé)
#   stack.sh down   arrête tout
#   stack.sh logs   dernières lignes des journaux
# À lancer depuis `nix develop` (toolchain Rust, libslic3r, orca-slicer, navigateurs Playwright).
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
stop() { if alive "$1"; then kill "$(cat "$RUN/$1.pid")" 2>/dev/null || true; sleep 1; fi; rm -f "$RUN/$1.pid"; }
wait_http() { # url timeout_s
	for _ in $(seq "$2"); do curl -fsS -o /dev/null "$1" 2>/dev/null && return 0; sleep 1; done
	return 1
}
src_hash() { find backend engine Cargo.toml Cargo.lock -type f \( -name '*.rs' -o -name '*.toml' -o -name '*.lock' -o -name '*.cpp' -o -name '*.h' \) -newer "$RUN/backend.built" 2>/dev/null | head -1; }

up_backend() {
	if [[ ! -f $RUN/backend.built ]] || [[ -n $(src_hash) ]] || ! alive backend; then
		echo "[stack] build backend…"
		cargo build --workspace --bins ${HARNESS_CARGO_FLAGS:-} >"$RUN/build.log" 2>&1 || { tail -60 "$RUN/build.log"; exit 1; }
		touch "$RUN/backend.built"
		stop backend
		local bin; bin=$(cargo metadata --format-version 1 --no-deps | python3 -c 'import json,sys;print(json.load(sys.stdin)["target_directory"])')/debug
		DATA_DIR=$STATE/data BIND=127.0.0.1:$BACK_PORT HARNESS_API=1 \
			INSTANCE_SECRET_KEY=${INSTANCE_SECRET_KEY:-harness-test-secret-key-0123456789abcdef} \
			RUST_LOG=${RUST_LOG:-warn} nohup "$bin/backend" >"$RUN/backend.log" 2>&1 &
		echo $! >"$RUN/backend.pid"
		wait_http "http://127.0.0.1:$BACK_PORT/api/health" 300 || { echo "[stack] backend ne répond pas"; tail -60 "$RUN/backend.log"; exit 1; }
	fi
}

up_frontend() {
	if ! alive frontend; then
		echo "[stack] front (vite dev)…"
		[[ -d frontend/node_modules ]] || (cd frontend && bun install --frozen-lockfile >"$RUN/bun.log" 2>&1)
		(cd frontend && BACKEND_URL=http://127.0.0.1:$BACK_PORT nohup bun run dev -- --host 127.0.0.1 --port "$FRONT_PORT" --strictPort \
			>"$RUN/frontend.log" 2>&1 & echo $! >"$RUN/frontend.pid")
	fi
	wait_http "http://127.0.0.1:$FRONT_PORT/" 240 || { echo "[stack] front ne répond pas"; tail -60 "$RUN/frontend.log"; exit 1; }
}

up_moonraker() {
	[[ ${HARNESS_MOONRAKER:-1} == 0 ]] && return 0
	command -v docker >/dev/null || { echo "[stack] docker absent : core.moonraker sera ignoré"; return 0; }
	if ! docker ps --format '{{.Names}}' | grep -qx ws-harness-moonraker; then
		docker rm -f ws-harness-moonraker >/dev/null 2>&1 || true
		docker run -d --name ws-harness-moonraker -p "127.0.0.1:$MOON_PORT:7125" \
			"${HARNESS_MOONRAKER_IMAGE:-ghcr.io/mainsail-crew/virtual-klipper-printer:master}" >/dev/null \
			|| { echo "[stack] impossible de lancer le Moonraker virtuel"; return 0; }
	fi
	wait_http "http://127.0.0.1:$MOON_PORT/server/info" 120 && echo "MOONRAKER_URL=http://127.0.0.1:$MOON_PORT" >>"$STATE/stack.env"
}

case ${1:-up} in
up)
	: >"$STATE/stack.env"
	up_backend
	up_frontend
	echo "BASE_URL=http://127.0.0.1:$FRONT_PORT" >>"$STATE/stack.env"
	up_moonraker
	[[ -d tests/oracle/node_modules ]] || (cd tests/oracle && bun install >"$RUN/oracle-install.log" 2>&1)
	echo "[stack] prête"
	;;
down)
	stop frontend; stop backend
	docker rm -f ws-harness-moonraker >/dev/null 2>&1 || true
	;;
logs)
	for f in build backend frontend; do echo "== $f"; tail -40 "$RUN/$f.log" 2>/dev/null || true; done
	;;
*) echo "usage: $0 up|down|logs"; exit 2 ;;
esac
