#!/usr/bin/env node
// Serveur Vite du front pour la pile E2E (lancé par harness/stack.sh, cwd = frontend/).
//
// Reprend frontend/vite.config.ts tel quel (plugins, proxy /api + /api/ws, warmup)
// et y superpose uniquement ce qui rend la pile déterministe pour Playwright :
//   - server.ws = false  : aucun canal WebSocket HMR, donc le serveur ne peut plus
//     pousser de `full-reload` (régénération de .svelte-kit/generated, ré-optimisation
//     de dépendances, fichier modifié pendant un test…) vers une page en cours de
//     test. `hmr: false` seul ne suffit pas : l'optimiseur de dépendances et la
//     reconnexion après redémarrage du serveur rechargent quand même la page.
//   - server.hmr = false : pas de mise à jour à chaud.
// Les modifications de sources restent prises en compte au chargement suivant
// (invalidation du graphe de modules) ; elles ne naviguent simplement plus la page.
// Le proxy /api/ws (flux d'événements de la file) n'utilise pas ce canal : il reste actif.
//
// Sans serveur WS, la poignée de main de @vite/client (ws://hôte/?token=…) resterait sans
// réponse : Chromium sérialise les connexions WebSocket en cours par hôte, et cette
// connexion pendante bloquerait ensuite /api/ws (progression du tranchage). Le plugin
// ci-dessous refuse donc immédiatement tout upgrade hors /api (réinstallé à chaque
// redémarrage interne de Vite, contrairement à un écouteur posé après createServer).
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

const front = process.cwd();
const { createServer } = await import(pathToFileURL(join(front, 'node_modules/vite/dist/node/index.js')).href);

const host = process.env.HARNESS_FRONTEND_HOST ?? '127.0.0.1';
const port = Number(process.env.HARNESS_FRONTEND_PORT ?? 15173);

const server = await createServer({
	configFile: join(front, 'vite.config.ts'),
	root: front,
	clearScreen: false,
	server: { host, port, strictPort: true, hmr: false, ws: false },
	plugins: [
		{
			name: 'harness-e2e-reject-hmr-socket',
			apply: 'serve',
			configureServer(s) {
				s.httpServer?.on('upgrade', (req, socket) => {
					if (req.url?.startsWith('/api')) return; // relayé par le proxy (ws: true)
					socket.end('HTTP/1.1 404 Not Found\r\nConnection: close\r\nContent-Length: 0\r\n\r\n');
				});
			}
		}
	]
});
await server.listen();
// PID réel du serveur pour stack.sh (setsid peut forker : $! n'est pas fiable).
if (process.env.HARNESS_FRONTEND_PIDFILE) writeFileSync(process.env.HARNESS_FRONTEND_PIDFILE, `${process.pid}\n`);
server.printUrls();
console.log(`[vite-e2e] prêt sur http://${host}:${port} (ws=off, hmr=off, pid=${process.pid})`);

let closing = false;
const shutdown = async () => {
	if (closing) return;
	closing = true;
	try {
		await server.close();
	} finally {
		process.exit(0);
	}
};
process.on('SIGTERM', shutdown);
process.on('SIGINT', shutdown);
