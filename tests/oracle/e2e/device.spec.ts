// Onglet Appareil sur le simulateur Moonraker (staging/native-parity/device/harness/moonraker-mock.mjs,
// à installer en harness/moonraker-mock.mjs) : consigne de température → valeur lue,
// déplacement manuel, ventilateur, fichier lancé → état Impression avec couche et
// progression qui avancent. Prérequis : backend avec les routes /files, /print, /gcode.
import { expect, test } from '@playwright/test';
import { bootstrap } from '../lib/app';

const MOONRAKER = process.env.MOONRAKER_URL ?? 'http://127.0.0.1:17125';

test('appareil : consignes, déplacement, ventilateur, lancement de fichier @F:ui.device', async ({ page }) => {
	test.setTimeout(180_000);
	await bootstrap(page);
	// Fichier présent sur l'imprimante (dépôt direct au simulateur).
	const up = await page.request.post(`${MOONRAKER}/server/files/upload`, {
		multipart: { root: 'gcodes', file: { name: 'appareil.gcode', mimeType: 'text/plain', buffer: Buffer.from('G28\n') } }
	});
	expect(up.ok()).toBe(true);

	// Imprimante déclarée par l'API (même preset machine que celle du bootstrap).
	const existing = await (await page.request.get('/api/printers')).json();
	const created = await page.request.post('/api/printers', {
		data: { name: 'Simulateur', moonraker_url: MOONRAKER, machine_preset_id: existing[0].machine_preset_id }
	});
	expect(created.status()).toBe(201);
	const id = (await created.json()).id as string;

	// API : consigne buse 200 °C → la température lue converge.
	expect((await page.request.post(`/api/printers/${id}/gcode`, { data: { script: 'M104 S200\nM140 S60' } })).status()).toBe(204);
	await expect
		.poll(async () => (await (await page.request.get(`/api/printers/${id}/status`)).json()).extruder_temp, { timeout: 30_000 })
		.toBeGreaterThan(190);
	const st = await (await page.request.get(`/api/printers/${id}/status`)).json();
	expect(st.extruder_target).toBe(200);
	expect(st.bed_target).toBe(60);

	// Déplacement + origine + ventilateur.
	expect((await page.request.post(`/api/printers/${id}/gcode`, { data: { script: 'G28\nG91\nG1 X10 F6000\nG90\nM106 S128' } })).status()).toBe(204);
	const st2 = await (await page.request.get(`/api/printers/${id}/status`)).json();
	expect(st2.homed_axes).toBe('xyz');
	expect(st2.fan_speed).toBeCloseTo(0.5, 1);
	expect(st2.webcam_url).toMatch(/\/webcam\/\?action=snapshot$/);

	// Fichiers + lancement → état Impression, couche et progression qui avancent.
	const files = await (await page.request.get(`/api/printers/${id}/files`)).json();
	expect(files.map((f: { path: string }) => f.path)).toContain('appareil.gcode');
	expect((await page.request.post(`/api/printers/${id}/print`, { data: { filename: 'appareil.gcode' } })).status()).toBe(204);
	await page.waitForTimeout(3000);
	const st3 = await (await page.request.get(`/api/printers/${id}/status`)).json();
	expect(st3.state).toBe('printing');
	expect(st3.print_duration).toBeGreaterThan(1);
	expect(st3.progress).toBeGreaterThan(0);
	expect(st3.current_layer).toBeGreaterThanOrEqual(1);
	expect(st3.total_layer).toBe(50);

	// UI : l'onglet Appareil affiche l'état et la couche (routes détectées).
	await page.goto(`/printers`);
	const panel = page.getByTestId('device-panel').first();
	if (await panel.count()) {
		await expect(page.getByTestId('device-state').first()).toContainText(/impression|printing/i, { timeout: 15_000 });
		await expect(page.getByTestId('device-layer').first()).toContainText('/ 50', { timeout: 15_000 });
	}
	await page.request.post(`/api/printers/${id}/cancel`);
});
