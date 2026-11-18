#!/usr/bin/env node
// Moonraker simulé pour l'oracle (core.moonraker, onglet Appareil) quand Docker
// (virtual-klipper) est indisponible. Sous-ensemble du contrat Moonraker utilisé
// par backend/src/adapters/moonraker :
//   GET  /server/info, /server/files/list?root=gcodes, /server/webcams/list
//   POST /server/files/upload (multipart, print=true)
//   GET  /printer/objects/query (print_stats + info couches, display_status,
//        extruder, heater_bed, fan, toolhead)
//   POST /printer/print/{start|pause|resume|cancel}, /printer/gcode/script
//   (+ objet exclude_object dans /printer/objects/query)
// Simulation : les températures convergent vers les consignes (M104/M109,
// M140/M190), le ventilateur suit M106/M107, une impression fait avancer
// durée, progression et couche (SIM_PRINT_SECONDS, défaut 120 s, 50 couches),
// G28 / G91+G1 mettent à jour toolhead.homed_axes / position.
// Objets (Klipper [exclude_object]) : les lignes EXCLUDE_OBJECT_DEFINE du
// fichier lancé (G-code d'Orca avec « Exclude objects ») peuplent
// exclude_object.objects ; EXCLUDE_OBJECT NAME=… les exclut ; l'objet courant
// tourne sur les objets non exclus pendant l'impression.
// Le WebSocket /websocket est refusé (le backend se replie sur l'interrogation).
//   node harness/moonraker-mock.mjs [port]   (défaut 17125, 127.0.0.1)
import { createServer } from 'node:http';

const PORT = Number(process.argv[2] || process.env.HARNESS_MOONRAKER_PORT || 17125);
const PRINT_SECONDS = Number(process.env.SIM_PRINT_SECONDS || 120);
const TOTAL_LAYERS = 50;
const files = new Map(); // path -> { size, modified, objects }
const excludeObject = { objects: [], excluded_objects: [], current_object: null };
const stats = { state: 'standby', filename: '', print_duration: 0 };
const heaters = {
	extruder: { temperature: 25, target: 0 },
	heater_bed: { temperature: 25, target: 0 }
};
const fan = { speed: 0 };
const toolhead = { homed_axes: '', position: [0, 0, 0, 0] };
const gcodeLog = [];
let relative = false;
let lastTick = Date.now();

/** Avance la simulation (températures, impression) depuis le dernier appel. */
function tick() {
	const now = Date.now();
	const dt = Math.min(10, (now - lastTick) / 1000);
	lastTick = now;
	for (const h of Object.values(heaters)) {
		// Convergence exponentielle (constante ~2 s) vers la consigne (ambiante 25 °C).
		const goal = h.target > 0 ? h.target : 25;
		h.temperature += (goal - h.temperature) * (1 - Math.exp(-dt / 2));
		if (Math.abs(goal - h.temperature) < 0.05) h.temperature = goal;
	}
	if (stats.state === 'printing') {
		stats.print_duration += dt;
		if (stats.print_duration >= PRINT_SECONDS) {
			stats.print_duration = PRINT_SECONDS;
			stats.state = 'complete';
		}
	}
}
/** État exclude_object ; objet courant = premier non exclu tant qu'on imprime. */
function currentExcludeObject() {
	const live = excludeObject.objects.filter((o) => !excludeObject.excluded_objects.includes(o.name));
	excludeObject.current_object = stats.state === 'printing' && live.length ? live[0].name : null;
	return JSON.parse(JSON.stringify(excludeObject));
}
const progress = () => (stats.filename ? Math.min(1, stats.print_duration / PRINT_SECONDS) : 0);
const currentLayer = () =>
	stats.state === 'standby' || !stats.filename ? 0 : Math.max(1, Math.ceil(progress() * TOTAL_LAYERS));

/** Arguments KEY=VALUE d'une commande étendue Klipper (clés en majuscules). */
function kv(args) {
	const out = {};
	for (const a of args) {
		const i = a.indexOf('=');
		if (i > 0) out[a.slice(0, i).toUpperCase()] = a.slice(i + 1);
	}
	return out;
}

/** EXCLUDE_OBJECT_DEFINE NAME= CENTER=x,y POLYGON=[[x,y],…] → objet Klipper. */
function defineObject(args) {
	const a = kv(args);
	if (!a.NAME) return;
	const name = a.NAME.toUpperCase();
	if (excludeObject.objects.some((o) => o.name === name)) return;
	const obj = { name };
	if (a.CENTER) obj.center = a.CENTER.split(',').map(Number);
	if (a.POLYGON) {
		try {
			obj.polygon = JSON.parse(a.POLYGON);
		} catch {
			obj.polygon = [];
		}
	}
	excludeObject.objects.push(obj);
}

/** Objets déclarés par un G-code (lignes EXCLUDE_OBJECT_DEFINE, casse conservée). */
function objectsOf(text) {
	const out = [];
	for (const raw of text.split('\n')) {
		const line = raw.split(';')[0].trim();
		if (/^EXCLUDE_OBJECT_DEFINE\b/i.test(line)) out.push(line.split(/\s+/).slice(1));
	}
	return out;
}

/** Nouvelle impression : objets du fichier, aucun exclu (comme Klipper au départ). */
function startObjects(path) {
	Object.assign(excludeObject, { objects: [], excluded_objects: [], current_object: null });
	for (const args of files.get(path)?.objects ?? []) defineObject(args);
}

/** Interprète un script G-code (lignes séparées par \n). */
function runGcode(script) {
	for (const raw of String(script).split('\n')) {
		const line = raw.split(';')[0].trim().toUpperCase();
		if (!line) continue;
		gcodeLog.push(line);
		const [cmd, ...args] = line.split(/\s+/);
		const arg = (k) => {
			const a = args.find((x) => x.startsWith(k));
			return a === undefined ? undefined : Number(a.slice(1));
		};
		if (cmd === 'M104' || cmd === 'M109') heaters.extruder.target = arg('S') ?? 0;
		else if (cmd === 'M140' || cmd === 'M190') heaters.heater_bed.target = arg('S') ?? 0;
		else if (cmd === 'M106') fan.speed = Math.max(0, Math.min(1, (arg('S') ?? 255) / 255));
		else if (cmd === 'M107') fan.speed = 0;
		else if (cmd === 'EXCLUDE_OBJECT_DEFINE') defineObject(raw.split(';')[0].trim().split(/\s+/).slice(1));
		else if (cmd === 'EXCLUDE_OBJECT') {
			const name = kv(args).NAME;
			if (name && !excludeObject.excluded_objects.includes(name)) {
				excludeObject.excluded_objects.push(name);
				if (excludeObject.current_object === name) excludeObject.current_object = null;
			}
		} else if (cmd === 'G90') relative = false;
		else if (cmd === 'G91') relative = true;
		else if (cmd === 'G28') {
			const axes = ['X', 'Y', 'Z'].filter((a) => args.length === 0 || args.some((x) => x.startsWith(a)));
			toolhead.homed_axes = [...new Set((toolhead.homed_axes + axes.join('')).toLowerCase())].sort().join('');
			axes.forEach((a) => (toolhead.position['XYZ'.indexOf(a)] = 0));
		} else if (cmd === 'G1' || cmd === 'G0') {
			['X', 'Y', 'Z'].forEach((a, i) => {
				const v = arg(a);
				if (v !== undefined) toolhead.position[i] = relative ? toolhead.position[i] + v : v;
			});
		}
	}
}

const json = (res, code, body) => {
	res.writeHead(code, { 'content-type': 'application/json' });
	res.end(JSON.stringify(body));
};

function readBody(req) {
	return new Promise((resolve, reject) => {
		const chunks = [];
		req.on('data', (c) => chunks.push(c));
		req.on('end', () => resolve(Buffer.concat(chunks)));
		req.on('error', reject);
	});
}

/** Parse multipart minimal : champs texte + fichier (nom, taille). */
function parseMultipart(buf, contentType) {
	const m = /boundary=(?:"([^"]+)"|([^;]+))/i.exec(contentType || '');
	if (!m) return null;
	const boundary = Buffer.from('--' + (m[1] || m[2]));
	const fields = {};
	let file = null;
	let pos = buf.indexOf(boundary);
	while (pos !== -1) {
		const start = pos + boundary.length;
		if (buf.slice(start, start + 2).toString() === '--') break;
		const headEnd = buf.indexOf('\r\n\r\n', start);
		if (headEnd === -1) break;
		const head = buf.slice(start, headEnd).toString();
		const next = buf.indexOf(boundary, headEnd);
		const body = buf.slice(headEnd + 4, next === -1 ? buf.length : next - 2);
		const name = /name="([^"]*)"/i.exec(head)?.[1];
		const filename = /filename="([^"]*)"/i.exec(head)?.[1];
		if (filename !== undefined) file = { field: name, filename, size: body.length, objects: objectsOf(body.toString('latin1')) };
		else if (name) fields[name] = body.toString();
		pos = next;
	}
	return { fields, file };
}

const server = createServer(async (req, res) => {
	const url = new URL(req.url, 'http://x');
	const p = url.pathname;
	tick();
	try {
		if (req.method === 'GET' && p === '/server/info')
			return json(res, 200, { result: { klippy_connected: true, klippy_state: 'ready', moonraker_version: 'mock-0.2' } });
		if (req.method === 'POST' && p === '/server/files/upload') {
			const parsed = parseMultipart(await readBody(req), req.headers['content-type']);
			if (!parsed?.file) return json(res, 400, { error: { code: 400, message: 'No file' } });
			const root = parsed.fields.root || 'gcodes';
			const path = parsed.fields.path ? parsed.fields.path + '/' + parsed.file.filename : parsed.file.filename;
			files.set(path, { size: parsed.file.size, modified: Date.now() / 1000, objects: parsed.file.objects });
			const start = String(parsed.fields.print).toLowerCase() === 'true';
			if (start) {
				Object.assign(stats, { state: 'printing', filename: path, print_duration: 0 });
				startObjects(path);
			}
			return json(res, 201, { item: { path, root }, print_started: start, print_queued: false, action: 'create_file' });
		}
		if (req.method === 'GET' && p === '/server/files/list')
			return json(res, 200, { result: [...files].map(([path, f]) => ({ path, size: f.size, modified: f.modified, permissions: 'rw' })) });
		if (req.method === 'GET' && p === '/server/webcams/list')
			return json(res, 200, {
				result: {
					webcams: [
						{
							name: 'printer',
							enabled: true,
							service: 'mjpegstreamer',
							stream_url: '/webcam/?action=stream',
							snapshot_url: '/webcam/?action=snapshot'
						}
					]
				}
			});
		if (req.method === 'GET' && p.startsWith('/webcam/')) {
			// Instantané : PNG 1×1 gris.
			res.writeHead(200, { 'content-type': 'image/png' });
			return res.end(
				Buffer.from(
					'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAAAAAA6fptVAAAACklEQVR4nGNgAAAAAgABSK+kcQAAAABJRU5ErkJggg==',
					'base64'
				)
			);
		}
		if (req.method === 'GET' && p === '/printer/objects/query')
			return json(res, 200, {
				result: {
					eventtime: Date.now() / 1000,
					status: {
						print_stats: {
							...stats,
							total_duration: stats.print_duration,
							info: { current_layer: currentLayer(), total_layer: stats.filename ? TOTAL_LAYERS : null }
						},
						display_status: { progress: progress() },
						extruder: { ...heaters.extruder },
						heater_bed: { ...heaters.heater_bed },
						fan: { ...fan },
						toolhead: { ...toolhead, position: [...toolhead.position] },
						exclude_object: currentExcludeObject()
					}
				}
			});
		if (req.method === 'POST' && p === '/printer/gcode/script') {
			let script = url.searchParams.get('script');
			if (script === null) {
				const body = (await readBody(req)).toString();
				try {
					script = JSON.parse(body || '{}').script ?? '';
				} catch {
					script = '';
				}
			}
			runGcode(script);
			return json(res, 200, { result: 'ok' });
		}
		// Journal des scripts reçus (diagnostic des tests).
		if (req.method === 'GET' && p === '/mock/gcode-log') return json(res, 200, { result: gcodeLog });
		const ctl = /^\/printer\/print\/(start|pause|resume|cancel)$/.exec(p);
		if (req.method === 'POST' && ctl) {
			stats.state = { start: 'printing', pause: 'paused', resume: 'printing', cancel: 'cancelled' }[ctl[1]];
			if (ctl[1] === 'start') {
				const filename = url.searchParams.get('filename');
				if (filename && !files.has(filename)) return json(res, 400, { error: { code: 400, message: 'File not found' } });
				stats.filename = filename || stats.filename;
				stats.print_duration = 0;
				startObjects(stats.filename);
			}
			return json(res, 200, { result: 'ok' });
		}
		json(res, 404, { error: { code: 404, message: 'Not Found' } });
	} catch (e) {
		json(res, 500, { error: { code: 500, message: String(e) } });
	}
});
server.on('upgrade', (_req, socket) => socket.end('HTTP/1.1 404 Not Found\r\n\r\n'));
server.listen(PORT, '127.0.0.1', () => console.log('[moonraker-mock] http://127.0.0.1:' + PORT));
