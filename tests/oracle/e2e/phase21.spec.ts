// Hôtes d'impression (10 types, PhysicalPrinterDialog / PrintHostSendDialog) et
// dialogue de maillage STEP (StepMeshDialog). Remplace phase20-lot17.spec.ts (réponses
// simulées par interception navigateur) : OctoPrint, PrusaLink (digest) et Repetier
// sont ici de vrais serveurs HTTP factices interrogés par le backend, dont on vérifie
// les requêtes reçues. Les assertions de lot17 (corps POST/PUT, clé conservée, champs
// requis, slug Repetier dans « Connexion… », Remailler inactif) sont reprises.
import { expect, test, type Page } from '@playwright/test';
import { createServer } from 'node:http';
import { createHash } from 'node:crypto';
import { join } from 'node:path';
import { bootstrap, importModel, REPO, sliceInUi } from '../lib/app';

type Hit = {
	method: string;
	url: string;
	headers: Record<string, string | string[] | undefined>;
	body: string;
};
type Send = (status: number, json: unknown, headers?: Record<string, string>) => void;
const md5 = (s: string) => createHash('md5').update(s).digest('hex');

function mockHost(handler: (r: Hit, send: Send) => void): Promise<{ url: string; log: Hit[]; close: () => void }> {
	return new Promise((ok) => {
		const log: Hit[] = [];
		const srv = createServer((req, res) => {
			const chunks: Buffer[] = [];
			req.on('data', (c: Buffer) => chunks.push(c));
			req.on('end', () => {
				const hit = {
					method: req.method ?? '',
					url: req.url ?? '',
					headers: req.headers,
					body: Buffer.concat(chunks).toString('latin1')
				};
				log.push(hit);
				handler(hit, (status, json, headers = {}) => {
					res.writeHead(status, {
						'content-type': 'application/json',
						...headers
					});
					res.end(JSON.stringify(json));
				});
			});
		});
		srv.listen(0, '127.0.0.1', () => {
			const a = srv.address() as { port: number };
			ok({ url: 'http://127.0.0.1:' + a.port, log, close: () => srv.close() });
		});
	});
}

const octoprint = () =>
	mockHost((r, send) => {
		if (r.headers['x-api-key'] !== 'OCTO-KEY') return send(403, { error: 'Invalid API key' });
		if (r.method === 'GET' && r.url === '/api/version')
			return send(200, {
				api: '0.1',
				server: '1.10.2',
				text: 'OctoPrint 1.10.2'
			});
		if (r.method === 'POST' && r.url === '/api/files/local') return send(201, { done: true });
		send(404, { error: 'not found' });
	});

const prusalink = () =>
	mockHost((r, send) => {
		const realm = 'Printer API';
		const nonce = 'n0nce';
		const auth = String(r.headers.authorization ?? '');
		const p = Object.fromEntries([...auth.matchAll(/(\w+)="?([^",]*)"?/g)].map((m) => [m[1], m[2]]));
		const ha1 = md5('maker:' + realm + ':secret');
		const ha2 = md5(r.method + ':' + p.uri);
		const expected = p.qop ? md5([ha1, nonce, p.nc, p.cnonce, p.qop, ha2].join(':')) : md5([ha1, nonce, ha2].join(':'));
		if (!auth.startsWith('Digest') || p.username !== 'maker' || p.response !== expected)
			return send(
				401,
				{ error: 'unauthorized' },
				{
					'www-authenticate': 'Digest realm="' + realm + '", nonce="' + nonce + '", qop="auth"'
				}
			);
		if (r.method === 'GET' && r.url === '/api/version')
			return send(200, {
				api: '2.0.0',
				server: '2.1.2',
				text: 'PrusaLink 2.1.2'
			});
		if (r.method === 'GET' && r.url === '/api/v1/storage')
			return send(200, {
				storage_list: [
					{
						path: '/local',
						name: 'Mémoire interne',
						available: true,
						read_only: false,
						free_space: 1e9
					},
					{
						path: '/usb',
						name: 'Clé USB',
						available: true,
						read_only: false,
						free_space: 5e9
					},
					{
						path: '/sd',
						name: 'SD protégée',
						available: true,
						read_only: true,
						free_space: 1e9
					}
				]
			});
		if (r.method === 'PUT' && r.url.startsWith('/api/v1/files/')) return send(201, {});
		send(404, { error: 'not found' });
	});

const repetier = () =>
	mockHost((r, send) => {
		if (r.headers['x-api-key'] !== 'REP-KEY') return send(401, { error: 'bad key' });
		if (r.url === '/printer/info')
			return send(200, {
				name: 'Repetier-Server',
				software: 'Repetier-Server',
				version: '1.4.15'
			});
		if (r.url === '/printer/list') return send(200, { data: [{ slug: 'ender3' }, { slug: 'mk3s' }] });
		if (r.url.startsWith('/printer/api/')) return send(200, { groupNames: ['#', 'Pieces'] });
		if (r.url.startsWith('/printer/job/') || r.url.startsWith('/printer/model/')) return send(200, {});
		send(404, { error: 'not found' });
	});

async function printerId(page: Page, name: string): Promise<string> {
	const list = (await (await page.request.get('/api/printers')).json()) as {
		id: string;
		name: string;
	}[];
	return list.find((p) => p.name === name)!.id;
}

test('OctoPrint (serveur factice) : capacités, clé refusée puis acceptée, envoi + démarrage @F:ui.print-hosts', async ({ page }) => {
	const octo = await octoprint();
	try {
		const project = await bootstrap(page);
		await page.goto('/printers');
		await page.getByTestId('printer-add').click();
		await page.getByTestId('host-type').selectOption('octoprint');
		const caps = page.getByTestId('host-capabilities');
		await expect(caps.locator('[data-capability="upload"]')).toHaveAttribute('data-ok', 'true');
		await expect(caps.locator('[data-capability="live"]')).toHaveAttribute('data-ok', 'false');
		await expect(page.getByTestId('host-help')).toContainText('Paramètres → API');
		await expect(page.getByTestId('host-limits')).toHaveCount(1);
		await page.locator('input[placeholder^="Imprimante du salon"]').fill('OctoPi');
		await expect(page.getByTestId('printer-save')).toBeDisabled(); // adresse requise
		await page.getByTestId('printer-moonraker-url').fill(octo.url.replace('http://', '')); // schéma ajouté
		await page.getByTestId('host-api-key').fill('MAUVAISE');
		const [post] = await Promise.all([
			page.waitForRequest((r) => r.url().endsWith('/api/printers') && r.method() === 'POST'),
			page.getByTestId('printer-save').click()
		]);
		expect(post.postDataJSON()).toMatchObject({
			host_type: 'octoprint',
			moonraker_url: octo.url,
			api_key: 'MAUVAISE'
		});
		const row = page.getByTestId('printer-row').filter({ hasText: 'OctoPi' });
		await expect(row.getByTestId('printer-credentials')).toHaveText('Clé API enregistrée');
		await expect(row.getByTestId('printer-status')).toHaveCount(0); // suivi Moonraker seulement

		// Clé refusée par l'hôte : échec affiché sur la carte, avec la cause.
		await row.getByTestId('printer-test').click();
		const result = row.getByTestId('printer-test-result');
		await expect(result).toHaveAttribute('data-ok', 'false');
		await expect(result).toContainText('Échec');
		expect(octo.log.at(-1)).toMatchObject({
			method: 'GET',
			url: '/api/version'
		});
		expect(octo.log.at(-1)!.headers['x-api-key']).toBe('MAUVAISE');

		// Bonne clé dans « Connexion… » : test réussi, libellé sans nom répété.
		await row.getByTestId('printer-connection').click();
		const dlg = page.getByTestId('printer-connection-dialog');
		await dlg.getByTestId('host-api-key').fill('OCTO-KEY');
		await dlg.getByTestId('connection-test').click();
		await expect(dlg.getByTestId('connection-result')).toHaveAttribute('data-ok', 'true');
		await expect(dlg.getByTestId('connection-result')).toHaveText('Connecté à OctoPrint 1.10.2');
		expect(octo.log.at(-1)!.headers['x-api-key']).toBe('OCTO-KEY');
		await dlg.getByTestId('connection-save').click();
		await expect(dlg).toHaveCount(0);
		await expect(row.getByTestId('printer-test-result')).toHaveAttribute('data-ok', 'true');

		// Champ clé laissé vide : PUT sans api_key, la clé enregistrée sert encore (l'hôte l'exige).
		const octoId = await printerId(page, 'OctoPi');
		await row.getByTestId('printer-connection').click();
		await expect(dlg.getByTestId('host-api-key')).toHaveAttribute('placeholder', /Enregistrée/);
		const [put] = await Promise.all([
			page.waitForRequest((r) => r.url().endsWith('/api/printers/' + octoId) && r.method() === 'PUT'),
			dlg.getByTestId('connection-test').click()
		]);
		expect(put.postDataJSON()).not.toHaveProperty('api_key');
		await expect(dlg.getByTestId('connection-result')).toHaveAttribute('data-ok', 'true');
		expect(octo.log.at(-1)!.headers['x-api-key']).toBe('OCTO-KEY');
		await dlg.getByTestId('connection-save').click();
		await expect(dlg).toHaveCount(0);

		// Envoi depuis l'éditeur : POST api/files/local multipart, print=true.
		await page.goto('/projects/' + project);
		await importModel(page, 'cube20');
		await sliceInUi(page);
		await page.getByTestId('send-to-printer').click();
		const send = page.getByTestId('send-dialog');
		await send.getByTestId('send-printer-select').selectOption(await printerId(page, 'OctoPi'));
		await expect(send.getByTestId('send-host-info')).toContainText('OctoPrint · envoi et lancement');
		await expect(send.getByTestId('send-storage')).toHaveCount(0);
		await send.getByTestId('send-start-now').check();
		await send.getByTestId('send-confirm').click();
		await expect(send.getByTestId('send-success')).toBeVisible({
			timeout: 30_000
		});
		const upload = octo.log.find((h) => h.method === 'POST' && h.url === '/api/files/local')!;
		expect(upload).toBeTruthy();
		expect(upload.headers['x-api-key']).toBe('OCTO-KEY');
		expect(upload.body).toMatch(/name="print"\r\n\r\ntrue/);
		expect(upload.body).toMatch(/name="file"; filename="[^"]+\.gcode"/);
		expect(upload.body).toContain('G1 ');
	} finally {
		octo.close();
	}
});

test('PrusaLink digest (serveur factice) : stockages à l envoi ; Repetier : imprimante du serveur choisie à l envoi @F:ui.print-hosts', async ({ page }) => {
	const [prusa, rep] = await Promise.all([prusalink(), repetier()]);
	try {
		const project = await bootstrap(page);
		// Repetier déclarés par l'API sans imprimante du serveur (host_port vide).
		const machine = (
			(await (await page.request.get('/api/printers')).json()) as {
				machine_preset_id: string;
			}[]
		)[0].machine_preset_id;
		for (const name of ['Ferme', 'Ferme 2'])
			await page.request.post('/api/printers', {
				data: {
					name,
					machine_preset_id: machine,
					host_type: 'repetier',
					moonraker_url: rep.url,
					api_key: 'REP-KEY'
				}
			});
		await page.goto('/printers');
		await page.getByTestId('printer-add').click();
		await page.getByTestId('host-type').selectOption('prusalink');
		await expect(page.getByTestId('host-capabilities').locator('[data-capability="storage"]')).toHaveAttribute('data-ok', 'true');
		await page.locator('input[placeholder^="Imprimante du salon"]').fill('MK4');
		await page.getByTestId('printer-moonraker-url').fill(prusa.url);
		await page.getByTestId('host-auth-user').click();
		await expect(page.getByTestId('printer-save')).toBeDisabled(); // utilisateur requis
		await page.getByTestId('host-user').fill('maker');
		await page.getByTestId('host-password').fill('secret');
		const [post] = await Promise.all([
			page.waitForRequest((r) => r.url().endsWith('/api/printers') && r.method() === 'POST'),
			page.getByTestId('printer-save').click()
		]);
		expect(post.postDataJSON()).toMatchObject({
			host_type: 'prusalink',
			host_auth: 'user',
			host_user: 'maker',
			host_password: 'secret'
		});
		const row = page.getByTestId('printer-row').filter({ hasText: 'MK4' });
		await expect(row.getByTestId('printer-credentials')).toContainText('Utilisateur « maker »');
		await expect(row.getByTestId('printer-credentials')).toContainText('Mot de passe enregistré');
		await row.getByTestId('printer-test').click();
		await expect(row.getByTestId('printer-test-result')).toHaveText('Connecté à PrusaLink 2.1.2');
		// Défi 401 puis réponse digest correcte (vérifiée par le serveur factice).
		expect(prusa.log.slice(0, 2).map((h) => [h.url, !!h.headers.authorization])).toEqual([
			['/api/version', false],
			['/api/version', true]
		]);
		// Le mot de passe n'est jamais renvoyé au client.
		const listed = await (await page.request.get('/api/printers')).text();
		expect(listed).not.toContain('secret');

		// Repetier dans « Connexion… » : liste des imprimantes du serveur (printer/list), slug enregistré.
		const ferme2 = page.getByTestId('printer-row').filter({ hasText: 'Ferme 2' });
		await expect(ferme2.getByTestId('printer-credentials')).toContainText('Imprimante du serveur à choisir');
		await ferme2.getByTestId('printer-connection').click();
		const dlg = page.getByTestId('printer-connection-dialog');
		await dlg.getByTestId('host-slug-list').click();
		await expect(dlg.locator('select[data-testid="host-slug"] option')).toHaveText(['ender3', 'mk3s']);
		expect(rep.log.some((h) => h.method === 'GET' && h.url === '/printer/list')).toBe(true);
		await dlg.getByTestId('host-slug').selectOption('mk3s');
		const [putSlug] = await Promise.all([
			page.waitForRequest((r) => /\/api\/printers\/[^/]+$/.test(r.url()) && r.method() === 'PUT'),
			dlg.getByTestId('connection-save').click()
		]);
		expect(putSlug.postDataJSON()).toMatchObject({
			host_type: 'repetier',
			host_port: 'mk3s',
			host_user: ''
		});
		expect(putSlug.postDataJSON()).not.toHaveProperty('api_key');
		await expect(ferme2.getByTestId('printer-host-type')).toHaveText('Repetier-Server');
		await expect(ferme2.getByTestId('printer-credentials')).toContainText('Imprimante « mk3s »');

		await page.goto('/projects/' + project);
		await importModel(page, 'cube20');
		await sliceInUi(page);
		await page.getByTestId('send-to-printer').click();
		const send = page.getByTestId('send-dialog');
		await send.getByTestId('send-printer-select').selectOption(await printerId(page, 'MK4'));
		const storage = send.getByTestId('send-storage');
		await expect(storage.locator('option')).toHaveText(['Mémoire interne', 'Clé USB']); // SD en lecture seule exclue
		await storage.selectOption('/usb');
		await send.getByTestId('send-start-now').check();
		await send.getByTestId('send-confirm').click();
		await expect(send.getByTestId('send-success')).toBeVisible({
			timeout: 30_000
		});
		// Défi 401 puis PUT authentifié (digest calculé pour la méthode PUT).
		const puts = prusa.log.filter((h) => h.method === 'PUT');
		expect(puts).toHaveLength(2);
		expect(puts[0].headers.authorization).toBeUndefined();
		const put = puts[1];
		expect(put.url).toMatch(/^\/api\/v1\/files\/usb\/[^/]+\.gcode$/);
		expect(put.headers['print-after-upload']).toBe('?1');
		expect(String(put.headers.authorization)).toMatch(/^Digest username="maker"/);

		// Repetier : envoi bloqué tant que l'imprimante du serveur n'est pas choisie.
		await send.getByTestId('send-printer-select').selectOption(await printerId(page, 'Ferme'));
		await expect(send.getByTestId('send-confirm')).toBeDisabled();
		const hostPrinter = send.getByTestId('send-host-printer');
		await expect(hostPrinter.locator('option:not([disabled])')).toHaveText(['ender3', 'mk3s']);
		await hostPrinter.selectOption('mk3s');
		await expect(send.getByTestId('send-confirm')).toBeEnabled();
		const ferme = (
			(await (await page.request.get('/api/printers')).json()) as {
				name: string;
				host_port?: string;
				moonraker_url?: string;
				has_api_key: boolean;
			}[]
		).find((p) => p.name === 'Ferme')!;
		expect(ferme).toMatchObject({
			host_port: 'mk3s',
			moonraker_url: rep.url,
			has_api_key: true
		});
		await expect(send.getByTestId('send-storage').locator('option')).toHaveText(['Default', 'Pieces']);
		expect(rep.log.some((h) => h.method === 'POST' && h.url === '/printer/api/mk3s')).toBe(true);
		await send.getByTestId('send-storage').selectOption('Pieces');
		await send.getByTestId('send-start-now').uncheck();
		await send.getByTestId('send-confirm').click();
		await expect(send.getByTestId('send-success')).toBeVisible({
			timeout: 30_000
		});
		const model = rep.log.find((h) => h.method === 'POST' && h.url === '/printer/model/mk3s')!;
		expect(model.body).toMatch(/name="group"\r\n\r\nPieces/);
		expect(model.body).toMatch(/name="a"\r\n\r\nupload/);
	} finally {
		prusa.close();
		rep.close();
	}
});

test('maillage STEP : préréglage, saisie bornée, pièces, mémorisation et « Ne plus afficher » @F:ui.step-mesh', async ({ page }) => {
	await bootstrap(page);
	const step = join(REPO, 'engine', 'tests', 'fixtures', 'step_mesh', 'compound.step');
	await page.getByTestId('import-input').setInputFiles(step);
	const dlg = page.getByTestId('step-mesh-dialog');
	await expect(dlg).toBeVisible({ timeout: 90_000 });
	await expect(dlg.getByTestId('step-preset-fine')).toHaveAttribute('aria-checked', 'true'); // défauts d'Orca
	await expect(dlg.getByTestId('step-done')).toHaveText('Terminé');
	await expect(dlg.getByTestId('step-apply')).toBeDisabled(); // réglages inchangés
	const before = Number((await dlg.getByTestId('step-triangles').textContent())!.replace(/\D/g, ''));

	await dlg.getByTestId('step-preset-light').click();
	await expect(dlg.getByTestId('step-linear-input')).toHaveValue('0,050');
	await expect(dlg.getByTestId('step-angle-input')).toHaveValue('1,00');
	await expect(dlg.getByTestId('step-state')).toHaveAttribute('data-dirty', 'true');
	await expect(dlg.getByTestId('step-done')).toHaveText('Appliquer et fermer');

	// Saisie hors bornes (Orca : 0,001 < linéaire < 0,1) refusée.
	await dlg.getByTestId('step-linear-input').fill('0,5');
	await dlg.getByTestId('step-linear-input').press('Enter');
	await expect(dlg.getByRole('alert')).toContainText('entre 0,001 et 0,1');
	await expect(dlg.getByTestId('step-apply')).toBeDisabled();
	await dlg.getByTestId('step-linear-input').fill('0,04');
	await dlg.getByTestId('step-linear-input').press('Enter');
	await expect(dlg.getByTestId('step-preset-custom')).toBeVisible();
	await dlg.getByTestId('step-split').check();

	const [res] = await Promise.all([
		page.waitForResponse((r) => r.url().includes('/step-mesh') && r.request().method() === 'POST'),
		dlg.getByTestId('step-apply').click()
	]);
	expect(res.request().postDataJSON()).toEqual({
		linear_deflection: 0.04,
		angle_deflection: 1,
		split_compound: true
	});
	const body = (await res.json()) as {
		triangles: number;
		objects: { name: string; triangles: number }[];
	};
	expect(body.objects).toHaveLength(2);
	await expect(dlg.getByTestId('step-triangles')).toHaveText(body.triangles.toLocaleString('fr-FR'));
	await expect(dlg.getByTestId('step-delta')).toContainText('vs import');
	expect(body.triangles).toBeLessThan(before);
	await expect(dlg.getByTestId('step-pieces').locator('li')).toHaveCount(2);
	await expect(dlg.getByTestId('step-pieces')).toContainText('SOLID-1');
	await expect(dlg.getByTestId('step-done')).toHaveText('Terminé');

	await dlg.getByTestId('step-dont-ask').check();
	await dlg.getByTestId('step-done').click();
	await expect(dlg).toHaveCount(0);
	const prefs = await page.evaluate(() => JSON.parse(localStorage.getItem('web-slicer.step-mesh') ?? 'null'));
	expect(prefs).toEqual({
		options: {
			linear_deflection: 0.04,
			angle_deflection: 1,
			split_compound: true
		},
		ask: false
	});

	// Import suivant : pas de dialogue, réglages mémorisés appliqués directement.
	const [auto] = await Promise.all([
		page.waitForRequest((r) => r.url().includes('/step-mesh') && r.method() === 'POST', { timeout: 90_000 }),
		page.getByTestId('import-input').setInputFiles(step)
	]);
	expect(auto.postDataJSON()).toEqual({
		linear_deflection: 0.04,
		angle_deflection: 1,
		split_compound: true
	});
	await expect(dlg).toHaveCount(0);
});
