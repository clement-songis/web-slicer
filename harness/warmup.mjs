#!/usr/bin/env node
// Préchauffage navigateur réel de la pile E2E (lancé par harness/stack.sh après
// l'installation des dépendances oracle ; relançable seul : `harness/stack.sh warmup`).
//
// Parcourt dans Chromium (Playwright de tests/oracle) les entrées que les tests
// atteignent à froid : /register (inscription réelle), /login (connexion réelle),
// /setup, puis un projet créé par /api/_harness/bootstrap. Pour chaque page :
// une seule navigation document, hydratation SvelteKit terminée, réseau au repos,
// puis une période de calme sans navigation secondaire.
//
// Une passe est « propre » si aucune navigation/rechargement secondaire, aucune
// requête de document/module avortée ou en 5xx (ex. 504 Outdated Optimize Dep),
// aucune erreur de page, et aucun événement d'optimiseur/rechargement Vite n'est
// apparu dans le journal du front pendant la passe. Les passes sont répétées
// jusqu'à une passe propre (HARNESS_WARMUP_PASSES, défaut 5), dans un délai borné
// (HARNESS_WARMUP_TIMEOUT secondes, défaut 300). Sinon : code 1 et diagnostic.
import { createRequire } from 'node:module';
import { closeSync, existsSync, openSync, readFileSync, readSync, statSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const HARNESS = dirname(fileURLToPath(import.meta.url));
const REPO = join(HARNESS, '..');
const ORACLE = join(REPO, 'tests', 'oracle');

const BASE = process.env.BASE_URL || `http://127.0.0.1:${process.env.HARNESS_FRONTEND_PORT || 15173}`;
const FRONT_LOG = process.env.HARNESS_FRONTEND_LOG || '';
const TIMEOUT_S = Number(process.env.HARNESS_WARMUP_TIMEOUT || 300);
const MAX_PASSES = Number(process.env.HARNESS_WARMUP_PASSES || 5);
const QUIET_MS = Number(process.env.HARNESS_WARMUP_QUIET_MS || 2500);
const STEP_MS = Number(process.env.HARNESS_WARMUP_STEP_TIMEOUT_MS || 45000);
const PASSWORD = 'harness-password-123';

const t0 = Date.now();
const deadline = t0 + TIMEOUT_S * 1000;
const elapsed = () => `${((Date.now() - t0) / 1000).toFixed(1)}s`;
const log = (m) => console.log(`[warmup ${elapsed()}] ${m}`);
function fail(msg, code = 1) {
	console.error(`[warmup] ÉCHEC : ${msg}`);
	process.exit(code);
}

// Garde-fou absolu : même si Playwright reste bloqué, le processus s'arrête.
setTimeout(() => fail(`délai global de ${TIMEOUT_S}s dépassé : la pile reste instable`, 3), TIMEOUT_S * 1000).unref();

let chromium;
try {
	({ chromium } = createRequire(join(ORACLE, 'package.json'))('playwright'));
} catch (e) {
	fail(`Playwright introuvable dans tests/oracle/node_modules (${e.message}) : lancer (cd tests/oracle && bun install)`);
}
const oracleCfg = JSON.parse(readFileSync(join(HARNESS, 'oracle.json'), 'utf8'));

// --- journal du front : événements Vite qui rechargeraient/invalideraient une page ---
const UNSTABLE = /optimiz|page reload|reloading|server restarted|restarting|outdated|error when starting|Internal server error/i;
function logSize() {
	try {
		return FRONT_LOG && existsSync(FRONT_LOG) ? statSync(FRONT_LOG).size : 0;
	} catch {
		return 0;
	}
}
function logSince(offset) {
	if (!FRONT_LOG || !existsSync(FRONT_LOG)) return [];
	const size = statSync(FRONT_LOG).size;
	if (size <= offset) return [];
	const fd = openSync(FRONT_LOG, 'r');
	const buf = Buffer.alloc(Math.min(size - offset, 1 << 20));
	readSync(fd, buf, 0, buf.length, offset);
	closeSync(fd);
	return buf
		.toString('utf8')
		.replace(/\x1b\[[0-9;]*m/g, '')
		.split('\n')
		.filter((l) => UNSTABLE.test(l));
}

function remaining() {
	return deadline - Date.now();
}
function stepTimeout() {
	const r = remaining() - 5000;
	if (r <= 0) throw new Error('délai global presque épuisé');
	return Math.min(STEP_MS, r);
}

async function hydrated(page) {
	// SvelteKit renseigne history.state['sveltekit:history'] au démarrage du client,
	// puis le routeur hydrate la page ; on attend ensuite que les modules aient fini
	// de charger (réseau au repos) et qu'aucune navigation ne survienne.
	await page.waitForFunction(() => history.state && history.state['sveltekit:history'] !== undefined, null, {
		timeout: stepTimeout()
	});
	await page.waitForLoadState('networkidle', { timeout: stepTimeout() }).catch(() => {});
}

async function quiet(page, probe) {
	const before = probe.docs;
	await page.waitForTimeout(QUIET_MS);
	if (probe.docs !== before) throw new Error(`navigation secondaire pendant la période de calme (${probe.docs - before} document(s) chargé(s))`);
}

async function visit(page, probe, path, ready) {
	const docs = probe.docs;
	const resp = await page.goto(path, { waitUntil: 'load', timeout: stepTimeout() });
	if (!resp) throw new Error(`goto ${path} : aucune réponse`);
	if (resp.status() >= 500) throw new Error(`goto ${path} → HTTP ${resp.status()}`);
	await hydrated(page);
	if (ready) await ready();
	await quiet(page, probe);
	const n = probe.docs - docs;
	if (n !== 1) throw new Error(`${path} : ${n} chargements de document au lieu d'un seul (rechargement/navigation secondaire)`);
}

async function submitAuth(page, probe, email) {
	const docs = probe.docs;
	await page.getByTestId('auth-email').fill(email, { timeout: stepTimeout() });
	await page.getByTestId('auth-password').fill(PASSWORD, { timeout: stepTimeout() });
	await page.getByTestId('auth-submit').click({ timeout: stepTimeout() });
	await page.getByTestId('user-menu').waitFor({ state: 'visible', timeout: stepTimeout() });
	await page.waitForLoadState('networkidle', { timeout: stepTimeout() }).catch(() => {});
	await quiet(page, probe);
	// Soumission traitée côté client : un rechargement trahirait une page non hydratée
	// (soumission native) ou un rechargement Vite pendant le formulaire.
	if (probe.docs !== docs) throw new Error(`soumission du formulaire : ${probe.docs - docs} chargement(s) de document (attendu : 0)`);
}

async function runPass(browser, n) {
	const offset = logSize();
	const ctx = await browser.newContext({ baseURL: BASE, viewport: { width: 1600, height: 1000 } });
	const page = await ctx.newPage();
	const probe = { docs: 0, problems: [] };
	page.on('domcontentloaded', () => probe.docs++);
	page.on('pageerror', (e) => probe.problems.push(`erreur de page : ${e.message.split('\n')[0]}`));
	page.on('requestfailed', (r) => {
		const type = r.resourceType();
		const err = r.failure()?.errorText ?? '?';
		if (r.isNavigationRequest() || type === 'script' || type === 'stylesheet')
			probe.problems.push(`requête ${type} avortée (${err}) : ${r.url()}`);
	});
	page.on('response', (r) => {
		const type = r.request().resourceType();
		if (r.status() >= 500 && (type === 'document' || type === 'script' || type === 'stylesheet'))
			probe.problems.push(`HTTP ${r.status()} sur ${type} : ${r.url()}`);
	});

	const email = `warmup${Date.now().toString(36)}${process.pid}p${n}@harness.local`;
	const steps = [];
	const step = async (name, fn) => {
		const s = Date.now();
		try {
			await fn();
			steps.push(`${name} ✓ ${((Date.now() - s) / 1000).toFixed(1)}s`);
		} catch (e) {
			steps.push(`${name} ✗`);
			throw new Error(`${name} : ${e.message.split('\n')[0]}`);
		}
	};
	let error = null;
	try {
		await step('/register', async () => {
			await visit(page, probe, '/register', () => page.getByTestId('auth-email').waitFor({ timeout: stepTimeout() }));
			await submitAuth(page, probe, email);
		});
		await step('/login', async () => {
			await ctx.clearCookies();
			await visit(page, probe, '/login', () => page.getByTestId('auth-email').waitFor({ timeout: stepTimeout() }));
			await submitAuth(page, probe, email);
		});
		await step('/setup', async () => {
			await visit(page, probe, '/setup');
		});
		await step('/projects/[bootstrap]', async () => {
			const res = await page.request.post('/api/_harness/bootstrap', {
				data: {
					email: `b${email}`,
					password: PASSWORD,
					printer: oracleCfg.printer,
					filament: oracleCfg.filament,
					process: oracleCfg.process
				},
				timeout: stepTimeout()
			});
			if (res.status() !== 200) throw new Error(`bootstrap → ${res.status()} : ${(await res.text()).slice(0, 300)}`);
			const { project_id } = await res.json();
			await visit(page, probe, `/projects/${project_id}`, () =>
				page.getByTestId('scene-canvas').waitFor({ state: 'visible', timeout: stepTimeout() })
			);
			// Le flux d'événements de la file passe par le proxy Vite (/api/ws) : il doit rester joignable.
			const ws = await page.evaluate(
				() =>
					new Promise((resolve) => {
						const sock = new WebSocket(location.origin.replace(/^http/, 'ws') + '/api/ws');
						const t = setTimeout(() => resolve('délai'), 5000);
						sock.onopen = () => {
							clearTimeout(t);
							sock.close();
							resolve('open');
						};
						sock.onerror = () => {
							clearTimeout(t);
							resolve('erreur');
						};
					})
			);
			if (ws !== 'open') throw new Error(`WebSocket /api/ws via le proxy Vite : ${ws}`);
		});
	} catch (e) {
		error = e.message;
	}
	await ctx.close().catch(() => {});
	const viteEvents = logSince(offset);
	const clean = !error && probe.problems.length === 0 && viteEvents.length === 0;
	return { clean, error, steps, problems: [...new Set(probe.problems)], viteEvents, docs: probe.docs };
}

log(`cible ${BASE} · ${MAX_PASSES} passe(s) max · délai ${TIMEOUT_S}s · journal front ${FRONT_LOG || '(aucun)'}`);
// La pile doit au moins répondre avant de lancer le navigateur.
for (;;) {
	try {
		const r = await fetch(BASE + '/', { signal: AbortSignal.timeout(5000) });
		if (r.status < 500) break;
	} catch {}
	if (remaining() < 10000) fail(`${BASE} ne répond pas`);
	await new Promise((r) => setTimeout(r, 1000));
}

let browser;
try {
	browser = await chromium.launch();
} catch (e) {
	fail(`lancement de Chromium impossible (${e.message.split('\n')[0]}) : lancer depuis nix develop (PLAYWRIGHT_BROWSERS_PATH)`);
}
const history = [];
let ok = false;
for (let n = 1; n <= MAX_PASSES; n++) {
	if (remaining() < 15000) break;
	const r = await runPass(browser, n);
	history.push(r);
	log(`passe ${n} : ${r.clean ? 'PROPRE' : 'instable'} · ${r.steps.join(' · ')}`);
	if (r.error) log(`  erreur : ${r.error}`);
	for (const p of r.problems.slice(0, 8)) log(`  navigateur : ${p}`);
	for (const v of r.viteEvents.slice(0, 8)) log(`  vite : ${v.trim()}`);
	if (r.clean) {
		ok = true;
		break;
	}
}
await browser.close().catch(() => {});
if (!ok) {
	const last = history.at(-1);
	fail(
		`aucune passe propre en ${history.length} passe(s) / ${elapsed()} — la pile reste instable. ` +
			`Dernière cause : ${last ? last.error || last.problems[0] || last.viteEvents[0] || '?' : 'aucune passe exécutée'}`
	);
}
log(`pile stable : passe propre (register, login, setup, projet) en ${elapsed()}`);
