// Phase 22 — manques d'interface de l'audit Orca (§10) : préférences complètes
// (caméra/souris, comportement, presets) persistées par utilisateur, modèles
// pratiques, détacher un preset, parcourir le réseau, conversion d'unités à
// l'import, À propos, origine avant déplacement (Appareil).
import { expect, test, type Page } from '@playwright/test';
import { writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawn, type ChildProcess } from 'node:child_process';
import { createServer as createNetServer } from 'node:net';
import { REPO, bootstrap, importModel, openParam, setControl, uniqueEmail } from '../lib/app';

const FILE = /^(file|fichier)$/i;
const HELP = /^(help|aide)$/i;
const MOONRAKER = process.env.MOONRAKER_URL ?? 'http://127.0.0.1:17125';

async function openPreferences(page: Page) {
	await page.getByRole('button', { name: FILE }).first().click();
	await page.getByRole('menuitem', { name: /pr[ée]f[ée]rences/i }).first().click();
	const dialog = page.getByTestId('preferences-dialog');
	await expect(dialog).toBeVisible();
	return dialog;
}

/** STL ASCII d'un cube de `side` mm. */
function cubeStl(side: number): string {
	const v = [[0,0,0],[1,0,0],[1,1,0],[0,1,0],[0,0,1],[1,0,1],[1,1,1],[0,1,1]].map((p) => p.map((c) => c * side));
	const f = [[0,2,1],[0,3,2],[4,5,6],[4,6,7],[0,1,5],[0,5,4],[1,2,6],[1,6,5],[2,3,7],[2,7,6],[3,0,4],[3,4,7]];
	let s = 'solid c\n';
	for (const t of f) {
		s += 'facet normal 0 0 0\nouter loop\n';
		for (const i of t) s += `vertex ${v[i].join(' ')}\n`;
		s += 'endloop\nendfacet\n';
	}
	return s + 'endsolid c\n';
}

test('préférences : onglets, caméra et comportement, persistées par utilisateur', async ({ page, browser }) => {
	await bootstrap(page);
	let dialog = await openPreferences(page);
	for (const tab of ['general', 'camera', 'behaviour', 'presets']) {
		await dialog.getByTestId(`pref-tab-${tab}`).click();
		await expect(dialog.getByTestId(`pref-tab-${tab}`)).toHaveAttribute('aria-selected', 'true');
	}
	await dialog.getByTestId('pref-tab-camera').click();
	// Défauts d'Orca : gauche = rotation, milieu / droit = panoramique.
	await expect(dialog.getByTestId('pref-leftDrag')).toHaveValue('rotate');
	await expect(dialog.getByTestId('pref-middleDrag')).toHaveValue('pan');
	await dialog.getByTestId('pref-leftDrag').selectOption('pan');
	await dialog.getByTestId('pref-rightDrag').selectOption('none');
	// Aucun bouton ne tourne la vue : avertissement.
	await expect(dialog.getByTestId('pref-no-rotate')).toBeVisible();
	await dialog.getByTestId('pref-rightDrag').selectOption('rotate');
	await expect(dialog.getByTestId('pref-no-rotate')).toHaveCount(0);
	await dialog.getByTestId('pref-zoom-to-mouse').check();
	await dialog.getByTestId('pref-reverse-wheel').check();
	await dialog.getByTestId('pref-orbit').fill('5');
	await dialog.getByTestId('pref-orbit').press('Enter');
	await expect(dialog.getByTestId('pref-orbit')).toHaveValue('2,00');
	// Pavé tactile : les boutons disparaissent, l'aide Alt/Maj s'affiche.
	await dialog.getByTestId('pref-camera-style-touchpad').click();
	await expect(dialog.getByTestId('pref-leftDrag')).toHaveCount(0);
	await expect(dialog).toContainText('Alt + déplacement');
	await dialog.getByTestId('pref-camera-style-default').click();
	await dialog.getByTestId('pref-tab-behaviour').click();
	await expect(dialog.getByTestId('pref-auto-arrange')).toBeChecked();
	await dialog.getByTestId('pref-auto-flush').selectOption('color');
	await dialog.getByTestId('pref-tab-presets').click();
	await dialog.getByTestId('pref-show-unsupported').check();
	await dialog.getByTestId('preferences-close').click();

	// Rechargement : tout est relu, sous la clé de l'utilisateur.
	await page.reload();
	await expect(page.getByTestId('scene-canvas')).toBeVisible();
	const keys = await page.evaluate(() => Object.keys(localStorage).filter((k) => k.startsWith('web-slicer.preferences:')));
	expect(keys).toHaveLength(1);
	dialog = await openPreferences(page);
	await dialog.getByTestId('pref-tab-camera').click();
	await expect(dialog.getByTestId('pref-leftDrag')).toHaveValue('pan');
	await expect(dialog.getByTestId('pref-rightDrag')).toHaveValue('rotate');
	await expect(dialog.getByTestId('pref-zoom-to-mouse')).toBeChecked();
	await expect(dialog.getByTestId('pref-reverse-wheel')).toBeChecked();
	await dialog.getByTestId('pref-tab-behaviour').click();
	await expect(dialog.getByTestId('pref-auto-flush')).toHaveValue('color');

	// Un autre compte, même navigateur : ses propres préférences (défauts).
	const storage = await page.context().storageState();
	const ctx2 = await browser.newContext({ storageState: { cookies: [], origins: storage.origins } });
	const page2 = await ctx2.newPage();
	await bootstrap(page2);
	const d2 = await openPreferences(page2);
	await d2.getByTestId('pref-tab-camera').click();
	await expect(d2.getByTestId('pref-leftDrag')).toHaveValue('rotate');
	await ctx2.close();
});

test('modèles pratiques : sous-menu, ajout, suggestion String Hell', async ({ page }) => {
	await bootstrap(page);
	const catalog = await (await page.request.get('/api/handy-models')).json();
	expect(catalog.length).toBe(9);
	await page.getByTestId('toolbar-add').click();
	await page.getByTestId('menu-sub-handy').click();
	await expect(page.getByRole('menuitem', { name: /Cube Voron|Voron Cube/ })).toBeVisible();
	await page.getByRole('menuitem', { name: /Cube Voron|Voron Cube/ }).click();
	await expect(page.getByTestId('object-row')).toHaveCount(1, { timeout: 60_000 });
	await expect(page.getByTestId('object-row').first()).toContainText(/Voron_Design_Cube|voron-cube/);

	// Combo Cube Orca : deux fichiers → deux objets, arrangés (aucun chevauchement).
	await page.getByTestId('toolbar-add').click();
	await page.getByTestId('menu-sub-handy').click();
	await page.getByRole('menuitem', { name: /Cube Orca|Orca Cube/ }).click();
	await expect(page.getByTestId('object-row')).toHaveCount(3, { timeout: 60_000 });

	// String Hell : la suggestion suit les valeurs du traitement édité.
	await page.getByTestId('toolbar-add').click();
	await page.getByTestId('menu-sub-handy').click();
	await page.getByRole('menuitem', { name: /String Hell/ }).click();
	await expect(page.getByTestId('object-row')).toHaveCount(4, { timeout: 60_000 });
	const project = await (await page.request.get(page.url().replace(/.*\/projects\//, '/api/projects/'))).json();
	const processId = project.active_presets?.process ?? project.process_preset_id;
	let applies: boolean | null = null;
	if (processId) {
		const r = await (await page.request.get(`/api/presets/${processId}/resolved`)).json();
		const v = (k: string) => String(Array.isArray(r.values?.[k]) ? r.values[k][0] : r.values?.[k] ?? '');
		applies = v('only_one_wall_top') === '1' && parseFloat(v('min_width_top_surface')) > 0;
	}
	if (applies === true) {
		await expect(page.getByTestId('handy-suggestion')).toBeVisible();
		await page.getByTestId('handy-suggestion-apply').click();
		await expect(page.getByTestId('handy-suggestion')).toHaveCount(0);
	} else if (applies === false) {
		await page.waitForTimeout(1500);
		await expect(page.getByTestId('handy-suggestion')).toHaveCount(0);
	}
});

test('conversion d’unités : objet de 1 mm → pouces, puis retour', async ({ page }) => {
	await bootstrap(page);
	const file = join(tmpdir(), `p22-tiny-${Date.now()}.stl`);
	writeFileSync(file, cubeStl(1));
	await page.getByTestId('import-input').setInputFiles(file);
	const dialog = page.getByTestId('unit-conversion-dialog');
	await expect(dialog).toBeVisible({ timeout: 60_000 });
	await expect(dialog).toHaveAttribute('data-suggested', 'inch');
	await expect(dialog).toContainText('1 × 1 × 1 mm');
	await expect(dialog.getByTestId('unit-convert-inch')).toContainText('25,4 × 25,4 × 25,4 mm');
	await dialog.getByTestId('unit-convert-inch').click();
	await expect(dialog).toHaveCount(0);
	await page.getByTestId('object-row').first().click({ button: 'right' });
	await expect(page.getByRole('menuitem', { name: /Revenir aux pouces|Restore to inches/ })).toBeVisible();
	await page.getByRole('menuitem', { name: /Revenir aux pouces|Restore to inches/ }).click();
	await page.getByTestId('object-row').first().click({ button: 'right' });
	await expect(page.getByRole('menuitem', { name: /Convertir depuis les mètres|Convert from meters/ })).toBeVisible();
	await page.keyboard.press('Escape');

	// « Ne plus proposer » : l'import suivant n'affiche plus le dialogue.
	await page.getByTestId('import-input').setInputFiles(file);
	await expect(dialog).toBeVisible({ timeout: 60_000 });
	await dialog.getByTestId('unit-dont-ask').check();
	await dialog.getByTestId('unit-keep').click();
	await page.getByTestId('import-input').setInputFiles(file);
	await expect(page.getByTestId('object-row')).toHaveCount(3, { timeout: 60_000 });
	await page.waitForTimeout(2500);
	await expect(dialog).toHaveCount(0);
});

test('détacher un preset système, À propos', async ({ page }) => {
	await bootstrap(page);
	await page.getByTestId('open-settings-print').click();
	const settings = page.getByTestId('settings-dialog');
	await expect(settings).toBeVisible();
	const original = (await settings.getByTestId('preset-name').textContent())?.trim() ?? '';
	await settings.getByTestId('preset-detach').click();
	const panel = settings.getByTestId('preset-detach-panel');
	await expect(panel).toContainText('copie autonome');
	const name = `Détaché ${Date.now()}`;
	await panel.getByTestId('preset-detach-name').fill(name);
	await panel.getByTestId('preset-detach-confirm').click();
	await expect(settings.getByTestId('preset-name')).toHaveText(name, { timeout: 20_000 });
	// Copie utilisateur sans parent : plus de bouton « Détacher ».
	await expect(settings.getByTestId('preset-detach')).toHaveCount(0);
	const list = await (await page.request.get('/api/presets?kind=process')).json();
	const created = list.find((p: { name: string }) => p.name === name);
	expect(created?.origin).toBe('user');
	expect(created?.inherits ?? null).toBeNull();
	expect(original).not.toBe(name);
	await page.keyboard.press('Escape');

	await page.getByRole('button', { name: HELP }).first().click();
	await page.getByRole('menuitem', { name: /à propos|about/i }).click();
	const about = page.getByTestId('about-dialog');
	await expect(about).toBeVisible();
	await expect(about.getByTestId('about-version')).toHaveText(/\d+\.\d+/);
	await expect(about.getByTestId('about-credits')).toContainText('AGPL-3.0');
});

test('parcourir le réseau (mDNS) dans le formulaire d’imprimante', async ({ page }) => {
	await bootstrap(page);
	await page.goto('/printers');
	await page.getByTestId('printer-add').click();
	await page.getByTestId('host-type').selectOption('octoprint');
	const discover = page.waitForResponse((r) => r.url().includes('/api/printers/discover?service=octoprint'), { timeout: 30_000 });
	await page.getByTestId('host-browse').click();
	expect((await discover).status()).toBe(200);
	const results = page.getByTestId('host-browse-results');
	await expect(results).toBeVisible();
	await expect(results).not.toContainText('Recherche des serveurs', { timeout: 30_000 });
	// Vraie recherche mDNS depuis le serveur : message explicite si rien n'est annoncé,
	// sinon « Utiliser » remplit l'adresse.
	await expect(results).toContainText(/Aucun serveur trouvé|Utiliser|Choisi/);
	const use = results.getByTestId('host-browse-use');
	if (await use.count()) {
		await use.first().click();
		await expect(page.getByTestId('printer-moonraker-url')).not.toHaveValue('');
	}
	// Types sans annonce DNS-SD : pas de bouton.
	await page.getByTestId('host-type').selectOption('repetier');
	await expect(page.getByTestId('host-browse')).toHaveCount(0);
});

test('hôtes Obico, Creality Print, Flashforge : champs et requête', async ({ page }) => {
	await bootstrap(page);
	await page.goto('/printers');
	await page.getByTestId('printer-add').click();
	// Flashforge : numéro de série + code d'accès (checkCode).
	await page.getByTestId('host-type').selectOption('flashforge');
	await expect(page.getByTestId('host-serial')).toBeVisible();
	await expect(page.getByTestId('host-slug')).toHaveCount(0);
	await page.getByTestId('printer-moonraker-url').fill('192.168.1.70');
	await page.getByTestId('host-api-key').fill('CODE42');
	await expect(page.getByText("Numéro de série requis avec le code d'accès.")).toBeVisible();
	await expect(page.getByTestId('printer-save')).toBeDisabled();
	await page.getByTestId('host-serial').fill('SNMOMC9900728');
	await expect(page.getByTestId('printer-save')).toBeEnabled();
	const req = page.waitForRequest((r) => r.url().endsWith('/api/printers') && r.method() === 'POST');
	await page.getByTestId('printer-save').click();
	const body = (await req).postDataJSON();
	expect(body).toMatchObject({
		host_type: 'flashforge',
		moonraker_url: 'http://192.168.1.70',
		api_key: 'CODE42',
		host_serial_number: 'SNMOMC9900728'
	});

	// Obico : « ID d'imprimante » (printhost_port) ; Creality Print : adresse + clé facultative.
	await page.getByTestId('host-type').selectOption('obico');
	await expect(page.getByText("ID d'imprimante")).toBeVisible();
	await expect(page.getByTestId('host-serial')).toHaveCount(0);
	await page.getByTestId('host-type').selectOption('crealityprint');
	await expect(page.getByTestId('host-help')).toContainText('K2');
	await expect(page.getByTestId('host-slug')).toHaveCount(0);
	// ElegooLink : protocole annoncé d'après le modèle du profil machine.
	await page.getByTestId('host-type').selectOption('elegoolink');
	await expect(page.getByTestId('host-elegoo-protocol')).toContainText(/OctoPrint|Centauri/);
	await expect(page.getByTestId('host-api-key')).toBeVisible();
	// Repetier : port vide → indication « depuis le profil machine » si le profil le porte.
	await page.getByTestId('host-type').selectOption('repetier');
	await expect(page.getByTestId('host-slug')).toHaveAttribute('placeholder', /depuis le profil machine|slug Repetier/);
});

test('tranchage : brouillon non enregistré et préférence haute/basse température envoyés', async ({ page }) => {
	await bootstrap(page);
	await importModel(page, 'cube20');
	// Préférence « autoriser les filaments haute et basse température ».
	const prefs = await openPreferences(page);
	await prefs.getByTestId('pref-tab-behaviour').click();
	await prefs.getByTestId('pref-mixed-temperature').check();
	await expect(prefs.getByTestId('pref-keep-painting')).not.toBeChecked();
	await prefs.getByTestId('preferences-close').click();
	// Modification non enregistrée du traitement : envoyée en brouillon.
	const control = await openParam(page, 'print', 'Quality', 'layer_height');
	await setControl(control, '0.16');
	await page.keyboard.press('Escape');
	const req = page.waitForRequest((r) => /\/api\/projects\/[^/]+\/slice$/.test(r.url()) && r.method() === 'POST');
	await page.getByTestId('slice-button').click();
	const body = (await req).postDataJSON();
	expect(body.allow_mixed_temperatures).toBe(true);
	expect(Number(body.draft?.process?.layer_height)).toBeCloseTo(0.16, 6);
});


/** Simulateur Moonraker privé (harness/moonraker-mock.mjs) : état neuf, non partagé. */
async function privateMoonraker(): Promise<{ url: string; proc: ChildProcess }> {
	const port = await new Promise<number>((ok) => {
		const s = createNetServer().listen(0, '127.0.0.1', () => {
			const p = (s.address() as { port: number }).port;
			s.close(() => ok(p));
		});
	});
	const proc = spawn(process.execPath, [join(REPO, 'harness', 'moonraker-mock.mjs'), String(port)], { stdio: 'pipe' });
	await new Promise<void>((ok, ko) => {
		proc.stdout?.on('data', (d: Buffer) => String(d).includes('moonraker-mock') && ok());
		proc.on('exit', () => ko(new Error('simulateur arrêté')));
	});
	return { url: `http://127.0.0.1:${port}`, proc };
}

async function declarePrinter(page: Page, name: string, url: string): Promise<string> {
	const existing = await (await page.request.get('/api/printers')).json();
	const created = await page.request.post('/api/printers', {
		data: { name, moonraker_url: url, machine_preset_id: existing[0].machine_preset_id }
	});
	expect(created.status()).toBe(201);
	return (await created.json()).id as string;
}

test('appareil : origine avant déplacement (simulateur neuf, sans origine)', async ({ page }) => {
	const mock = await privateMoonraker();
	try {
		await bootstrap(page);
		await declarePrinter(page, 'Sans origine', mock.url);
		await page.getByRole('button', { name: /^(appareil|device)$/i }).first().click();
		const panel = page.getByTestId('device-panel');
		await panel.getByText('Sans origine').first().click();
		await expect(panel.getByTestId('device-unhomed')).toContainText('X Y Z', { timeout: 30_000 });
		await panel.getByRole('button', { name: /^X\+10 mm$/ }).click();
		const warn = panel.getByTestId('device-recenter');
		await expect(warn).toBeVisible();
		await warn.getByTestId('device-recenter-home').click();
		// Le simulateur a reçu G28 : plus aucun axe à mettre à l'origine.
		await expect(panel.getByTestId('device-unhomed')).toHaveCount(0, { timeout: 30_000 });
		const st = await (await page.request.get(`${mock.url}/printer/objects/query?toolhead`)).json();
		expect(st.result.status.toolhead.homed_axes).toBe('xyz');
		await panel.getByRole('button', { name: /^Y\+10 mm$/ }).click();
		await expect(warn).toHaveCount(0);
	} finally {
		mock.proc.kill();
	}
});

test('appareil : objets de l’impression, exclusion pendant l’impression (simulateur réel)', async ({ page }) => {
	const mock = await privateMoonraker();
	try {
		await bootstrap(page);
		await declarePrinter(page, 'Simulateur objets', mock.url);
		// G-code étiqueté comme Orca (« Exclude objects ») envoyé et lancé sur le simulateur.
		const gcode = [
			'EXCLUDE_OBJECT_DEFINE NAME=cube.stl_id_0_copy_0 CENTER=100,100 POLYGON=[[90,90],[110,90],[110,110],[90,110]]',
			'EXCLUDE_OBJECT_DEFINE NAME=cube.stl_id_0_copy_1 CENTER=140,100 POLYGON=[[130,90],[150,90],[150,110],[130,110]]',
			'G28',
			...Array.from({ length: 400 }, (_, i) => `G1 X${100 + (i % 10)} Y100 F600`)
		].join('\n');
		const up = await page.request.post(`${mock.url}/server/files/upload`, {
			multipart: { root: 'gcodes', print: 'true', file: { name: 'etiquete.gcode', mimeType: 'text/plain', buffer: Buffer.from(gcode) } }
		});
		expect(up.ok()).toBe(true);
		await page.getByRole('button', { name: /^(appareil|device)$/i }).first().click();
		const panel = page.getByTestId('device-panel');
		await panel.getByText('Simulateur objets').first().click();
		const section = panel.getByTestId('device-objects');
		await expect(section.getByTestId('device-object')).toHaveCount(2, { timeout: 30_000 });
		await expect(section.getByTestId('device-objects-map').locator('polygon')).toHaveCount(2);
		await expect(section.getByTestId('device-object').first()).toHaveAttribute('data-state', 'current');
		await expect(section.getByTestId('device-object').nth(1)).toContainText('copie 2');
		await section.getByTestId('device-object').nth(1).getByTestId('device-object-exclude').click();
		await expect(section).toContainText('Action irréversible');
		await section.getByTestId('device-object-exclude-confirm').click();
		await expect(section.getByTestId('device-object').nth(1)).toHaveAttribute('data-state', 'excluded');
		await expect(section).toContainText('1 exclu');
		const q = await (await page.request.get(`${mock.url}/printer/objects/query?exclude_object`)).json();
		expect(q.result.status.exclude_object.excluded_objects.map((s: string) => s.toLowerCase())).toContain(
			'cube.stl_id_0_copy_1'
		);
	} finally {
		mock.proc.kill();
	}
});

test('réglages du plateau : filaments par buse (Bambu Lab H2D, 2 buses)', async ({ page }) => {
	const res = await page.request.post('/api/_harness/bootstrap', {
		data: {
			email: uniqueEmail(),
			password: 'harness-password-123',
			printer: 'Bambu Lab H2D 0.4 nozzle',
			filament: 'Bambu PLA Basic @BBL H2D',
			process: '0.20mm Standard @BBL H2D'
		}
	});
	expect(res.status(), await res.text()).toBe(200);
	const { project_id } = await res.json();
	await page.goto(`/projects/${project_id}`);
	await expect(page.getByTestId('scene-canvas')).toBeVisible();
	await page.getByTestId('add-filament-slot').click();
	await page.getByTestId('plate-settings-button').first().click();
	const dialog = page.getByTestId('settings-dialog');
	const section = dialog.getByTestId('plate-filament-map');
	await expect(section).toBeVisible({ timeout: 20_000 });
	await section.getByTestId('plate-filament-map-mode').selectOption('Manual');
	await expect(section.getByTestId('plate-filament-map-1-1')).toHaveText('Gauche');
	await section.getByTestId('plate-filament-map-2-2').click();
	await dialog.getByTestId('settings-apply').click();
	await page.keyboard.press('Control+s');
	await expect
		.poll(async () => JSON.stringify((await (await page.request.get(`/api/projects/${project_id}`)).json()).scene), {
			timeout: 20_000
		})
		.toMatch(/"filament_map_mode":"Manual"[\s\S]*"filament_map":\["1","2"\]|"filament_map":\["1","2"\][\s\S]*"filament_map_mode":"Manual"/);
});
