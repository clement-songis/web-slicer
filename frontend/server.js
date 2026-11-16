// Serveur de production : handler adapter-node (`build/`) + relais de l'upgrade
// WebSocket `/api/ws` vers le backend (le flux d'événements de la file : fin de
// conversion des imports, progression du tranchage). Les requêtes HTTP `/api/*`
// sont relayées par `src/hooks.server.ts`. Sans dépendance.
//
//   bun run build && PORT=3000 ORIGIN=https://slicer.example BACKEND_URL=http://127.0.0.1:8080 node server.js
import http from 'node:http';
import net from 'node:net';
import { handler } from './build/handler.js';

const backend = new URL(process.env.BACKEND_URL ?? 'http://127.0.0.1:8080');
const port = Number(process.env.PORT ?? 3000);
const host = process.env.HOST ?? '0.0.0.0';

const server = http.createServer(handler);

server.on('upgrade', (req, socket, head) => {
	if (!req.url?.startsWith('/api/')) {
		socket.destroy();
		return;
	}
	const upstream = net.connect(Number(backend.port || 80), backend.hostname, () => {
		const lines = [`${req.method} ${req.url} HTTP/${req.httpVersion}`];
		for (let i = 0; i < req.rawHeaders.length; i += 2) {
			const name = req.rawHeaders[i];
			lines.push(
				`${name}: ${name.toLowerCase() === 'host' ? backend.host : req.rawHeaders[i + 1]}`
			);
		}
		upstream.write(lines.join('\r\n') + '\r\n\r\n');
		if (head?.length) upstream.write(head);
		upstream.pipe(socket);
		socket.pipe(upstream);
	});
	const close = () => {
		upstream.destroy();
		socket.destroy();
	};
	upstream.on('error', close);
	socket.on('error', close);
});

server.listen(port, host, () =>
	console.log(`Listening on http://${host}:${port} (backend ${backend.origin})`)
);
