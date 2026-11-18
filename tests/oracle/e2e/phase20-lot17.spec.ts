// Phase 20 (lot 17) : hôtes d'impression (PhysicalPrinterDialog) et maillage STEP
// (StepMeshDialog). Les hôtes non Moonraker sont simulés par interception des
// routes /api/printers/{id}/(test|storages|host-printers) côté navigateur ; le
// corps envoyé au serveur réel (POST/PUT /api/printers) est vérifié.
import { expect, test } from '@playwright/test';
import { join } from 'node:path';
import { bootstrap, REPO } from '../lib/app';

const STEP = join(REPO, 'engine', 'tests', 'fixtures', 'step_mesh');

test('hôte OctoPrint : type, clé conservée, test, pas de suivi Moonraker @F:ui.print-hosts', async ({ page }) => {
	await bootstrap(page);
	await page.goto('/printers');
	await page.getByTestId('printer-add').click();
	await page.getByTestId('host-type').selectOption('octoprint');
	await expect(page.getByTestId('host-limits')).toBeVisible();
	await expect(page.getByTestId('printer-save')).toBeDisabled(); // adresse requise
	await page.getByTestId('printer-moonraker-url').fill('octopi.local');
	await page.getByTestId('host-api-key').fill('CLE-123');
	const [req] = await Promise.all([
		page.waitForRequest((r) => r.url().endsWith('/api/printers') && r.method() === 'POST'),
		page.getByTestId('printer-save').click()
	]);
	expect(req.postDataJSON()).toMatchObject({ host_type: 'octoprint', moonraker_url: 'http://octopi.local', api_key: 'CLE-123' });
	const row = page.getByTestId('printer-row').filter({ hasText: 'octopi.local' });
	await expect(row.getByTestId('printer-host-type')).toHaveText('OctoPrint');
	await expect(row.getByTestId('printer-status')).toHaveCount(0); // état Moonraker seulement
	const printers = await (await page.request.get('/api/printers')).json();
	const p = printers.find((x: { moonraker_url?: string }) => x.moonraker_url === 'http://octopi.local');
	expect(p).toMatchObject({ host_type: 'octoprint', has_api_key: true });

	// Connexion… : clé vide = conservée ; test de connexion (hôte simulé).
	await page.route(`**/api/printers/${p.id}/test`, (r) =>
		r.fulfill({ json: { connected: true, klippy_state: 'OctoPrint', moonraker_version: '1.10.2' } })
	);
	await row.getByTestId('printer-connection').click();
	const dlg = page.getByTestId('printer-connection-dialog');
	await expect(dlg.getByTestId('host-type')).toHaveValue('octoprint');
	await expect(dlg.getByTestId('host-api-key')).toHaveAttribute('placeholder', /Enregistrée/);
	const [put] = await Promise.all([
		page.waitForRequest((r) => r.url().endsWith(`/api/printers/${p.id}`) && r.method() === 'PUT'),
		dlg.getByTestId('connection-test').click()
	]);
	expect(put.postDataJSON()).not.toHaveProperty('api_key');
	await expect(dlg.getByTestId('connection-result')).toHaveText('Connecté à OctoPrint 1.10.2');
	await dlg.getByTestId('connection-save').click();
	await expect(dlg).toHaveCount(0);
	const after = await (await page.request.get('/api/printers')).json();
	expect(after.find((x: { id: string }) => x.id === p.id).has_api_key).toBe(true);
});

test('PrusaLink utilisateur / mot de passe, Repetier : liste des imprimantes du serveur @F:ui.print-hosts', async ({ page }) => {
	await bootstrap(page);
	await page.goto('/printers');
	await page.getByTestId('printer-add').click();
	await page.getByTestId('host-type').selectOption('prusalink');
	await page.getByTestId('printer-moonraker-url').fill('http://192.168.1.30');
	await page.getByTestId('host-auth-user').click();
	await expect(page.getByTestId('printer-save')).toBeDisabled(); // utilisateur requis
	await page.getByTestId('host-user').fill('maker');
	await page.getByTestId('host-password').fill('secret');
	const [req] = await Promise.all([
		page.waitForRequest((r) => r.url().endsWith('/api/printers') && r.method() === 'POST'),
		page.getByTestId('printer-save').click()
	]);
	expect(req.postDataJSON()).toMatchObject({ host_type: 'prusalink', host_auth: 'user', host_user: 'maker', host_password: 'secret' });
	await expect(page.getByTestId('printer-row').filter({ hasText: '192.168.1.30' })).toBeVisible();
	const list = await (await page.request.get('/api/printers')).json();
	expect(list.find((x: { host_type: string }) => x.host_type === 'prusalink')).toMatchObject({ host_auth: 'user', host_user: 'maker', has_host_password: true });

	// Repetier : passage du type dans « Connexion… », slug choisi dans la liste.
	const row = page.getByTestId('printer-row').filter({ hasText: '192.168.1.30' });
	const id = await row.getAttribute('data-printer-id');
	await page.route(`**/api/printers/${id}/host-printers`, (r) => r.fulfill({ json: ['ender3', 'mk3s'] }));
	await row.getByTestId('printer-connection').click();
	const dlg = page.getByTestId('printer-connection-dialog');
	await dlg.getByTestId('host-type').selectOption('repetier');
	await dlg.getByTestId('printer-moonraker-url').fill('http://repetier.local:3344');
	await dlg.getByTestId('host-slug-list').click();
	await expect(dlg.locator('select[data-testid="host-slug"] option')).toHaveCount(2);
	await dlg.getByTestId('host-slug').selectOption('mk3s');
	const [put] = await Promise.all([
		page.waitForRequest((r) => r.url().endsWith(`/api/printers/${id}`) && r.method() === 'PUT'),
		dlg.getByTestId('connection-save').click()
	]);
	expect(put.postDataJSON()).toMatchObject({ host_type: 'repetier', host_port: 'mk3s', host_user: '' });
	await expect(row.getByTestId('printer-host-type')).toHaveText('Repetier-Server');
});

test('maillage STEP : dialogue après import, déflexions, découpe, triangles @F:ui.step-mesh', async ({ page }) => {
	await bootstrap(page);
	await page.getByTestId('import-input').setInputFiles(join(STEP, 'compound.step'));
	const dlg = page.getByTestId('step-mesh-dialog');
	await expect(dlg).toBeVisible({ timeout: 90_000 });
	const before = Number((await dlg.getByTestId('step-triangles').textContent())!.replace(/\s/g, ''));
	expect(before).toBeGreaterThan(0);
	await expect(dlg.getByTestId('step-apply')).toBeDisabled(); // réglages inchangés
	await dlg.getByTestId('step-linear').fill('0.1');
	await dlg.getByTestId('step-angle').fill('1');
	await dlg.getByTestId('step-split').check();
	const [res] = await Promise.all([
		page.waitForResponse((r) => r.url().includes('/step-mesh') && r.request().method() === 'POST'),
		dlg.getByTestId('step-apply').click()
	]);
	const body = await res.json();
	expect(res.request().postDataJSON()).toEqual({ linear_deflection: 0.1, angle_deflection: 1, split_compound: true });
	expect(body.objects.length).toBe(2);
	await expect(dlg.getByTestId('step-pieces')).toContainText('2 pièces');
	await expect(dlg.getByTestId('step-triangles')).toHaveText(body.triangles.toLocaleString('fr-FR').replace(/\u202f/g, '\u202f'));
	expect(body.triangles).toBeLessThan(before);
	await dlg.getByTestId('step-done').click();
	await expect(dlg).toHaveCount(0);
	await expect(page.getByTestId('object-row')).toHaveCount(1);
});
