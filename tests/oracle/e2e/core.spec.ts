// Parcours de bout en bout (gelé). Doit rester vert en permanence.
import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import {
	bootstrap, downloadGcode, harnessSlice, importModel, openParam, oracleCfg, readJsonRef, readRef,
	REPO, setControl, slug, sliceInUi, uniqueEmail, type FlatConfig
} from '../lib/app';
import { configBlock, expectSameGcode, extrusionRoles, layerCount } from '../lib/gcode';

const MODEL = oracleCfg.param_model;
const needRefs = () => {
	const ref = readRef('base.gcode');
	const base = readJsonRef<FlatConfig>('base_config.json');
	test.skip(!ref || !base, 'références Orca absentes : lancer tests/oracle/tools/make_refs.py');
	return { ref: ref!, base: base! };
};

test('API de harness : G-code identique à orca-slicer CLI @F:core.harness-api', async ({ request }) => {
	const { ref, base } = needRefs();
	expectSameGcode(await harnessSlice(request, MODEL, base), ref);
});

test('API de harness : config invalide refusée comme Orca @F:core.harness-api', async ({ request }) => {
	const { base } = needRefs();
	const res = await request.post('/api/_harness/slice', {
		multipart: {
			model: { name: `${MODEL}.stl`, mimeType: 'model/stl', buffer: readFileSync(join(REPO, 'tests/oracle/fixtures', `${MODEL}.stl`)) },
			config: JSON.stringify({ ...base, layer_height: '5' })
		}
	});
	expect(res.status()).toBe(422);
});

test('inscription puis connexion depuis l’UI @F:core.auth-ui', async ({ page, context }) => {
	const email = uniqueEmail();
	await page.goto('/register');
	await page.getByTestId('auth-email').fill(email);
	await page.getByTestId('auth-password').fill('harness-password-123');
	await page.getByTestId('auth-submit').click();
	await expect(page.getByTestId('user-menu')).toBeVisible();
	await context.clearCookies();
	await page.goto('/login');
	await page.getByTestId('auth-email').fill(email);
	await page.getByTestId('auth-password').fill('harness-password-123');
	await page.getByTestId('auth-submit').click();
	await expect(page.getByTestId('user-menu')).toBeVisible();
});

test('import STL : objet listé et scène affichée @F:core.import', async ({ page }) => {
	await bootstrap(page);
	await importModel(page, MODEL);
	await expect(page.getByTestId('object-row').first()).toContainText(MODEL);
	await expect(page.getByTestId('scene-canvas')).toBeVisible();
});

test('réglages : tous les onglets et options Orca présents (mode expert) @F:core.settings-ui', async ({ page }) => {
	type Tab = { title: string; defined_in?: string; sections: { title: string; options: (string | object)[] }[] };
	const ui = JSON.parse(readFileSync(join(REPO, 'audit/ui_inventory.json'), 'utf8')) as { settings_tabs: Tab[] };
	const params = JSON.parse(readFileSync(join(REPO, 'audit/parameters.json'), 'utf8')).parameters as Record<string, unknown>;
	const scopeOf = (t: Tab) => (t.defined_in?.startsWith('TabFilament') ? 'filament' : t.defined_in?.startsWith('TabPrinter') ? 'printer' : 'print');
	await bootstrap(page);
	const missing: string[] = [];
	for (const scope of ['print', 'filament', 'printer']) {
		await page.getByTestId(`open-settings-${scope}`).click();
		const dialog = page.getByTestId('settings-dialog');
		await expect(dialog).toBeVisible();
		await dialog.getByTestId('settings-mode-develop').click();
		const tabs = ui.settings_tabs.filter((t) => scopeOf(t) === scope && !/^TabPrint(Model|Plate)/.test(t.defined_in ?? ''));
		for (const tab of tabs) {
			// « page_name » = page extrudeur dynamique d'Orca (Extruder 1 en mono-extrudeur).
			const title = tab.title === 'page_name' ? 'Extruder 1' : tab.title;
			const tabBtn = dialog.getByTestId(`settings-tab-${slug(title)}`);
			if (!(await tabBtn.count())) { missing.push(`onglet ${scope}/${title}`); continue; }
			await tabBtn.click();
			const present = new Set(await dialog.locator('[data-param]').evaluateAll((els) => els.map((e) => e.getAttribute('data-param'))));
			for (const s of tab.sections) for (const o of s.options) {
				if (typeof o !== 'string') continue; // option dynamique (non résolue par l'audit)
				const key = o.split('#')[0];
				if (key in params && !present.has(key)) missing.push(`${scope}/${title}/${s.title}: ${key}`);
			}
		}
		await page.keyboard.press('Escape');
	}
	expect(missing, `options manquantes (${missing.length}) :\n${missing.slice(0, 40).join('\n')}`).toEqual([]);
});

test('tranchage depuis l’UI : aperçu avec le même nombre de couches qu’Orca @F:core.slice-ui', async ({ page }) => {
	const { ref } = needRefs();
	await bootstrap(page);
	await importModel(page, MODEL);
	expect(await sliceInUi(page)).toBe(layerCount(ref));
});

test('export G-code depuis l’UI identique à Orca et à l’API de harness @F:core.export-gcode', async ({ page }) => {
	const { ref, base } = needRefs();
	await bootstrap(page);
	await importModel(page, MODEL);
	await sliceInUi(page);
	const fromUi = await downloadGcode(page);
	expectSameGcode(fromUi, ref, 'G-code exporté depuis l’UI');
	expectSameGcode(fromUi, await harnessSlice(page.request, MODEL, base), 'UI vs API de harness');
});

test('aperçu : curseur de couches et légende des rôles d’extrusion @F:core.preview', async ({ page }) => {
	const { ref } = needRefs();
	await bootstrap(page);
	await importModel(page, MODEL);
	await sliceInUi(page);
	const slider = page.getByTestId('layer-slider');
	await slider.fill('1');
	await expect(slider).toHaveAttribute('data-current-layer', '1');
	const roles = await page.getByTestId('legend-item').evaluateAll((els) => els.map((e) => e.getAttribute('data-role')));
	for (const r of extrusionRoles(ref)) expect(roles, `rôle « ${r} » absent de la légende`).toContain(r);
});

test('sauvegarde projet : scène et réglage modifié restaurés après rechargement @F:core.project-save', async ({ page }) => {
	await bootstrap(page);
	await importModel(page, MODEL);
	const ctl = await openParam(page, 'print', 'Strength', 'wall_loops');
	expect(await setControl(ctl, '5')).toBe(true);
	await page.getByTestId('settings-apply').click();
	await page.getByTestId('save-project').click();
	await page.reload();
	await expect(page.getByTestId('object-row')).toHaveCount(1);
	await expect(await openParam(page, 'print', 'Strength', 'wall_loops')).toHaveValue('5');
	await page.keyboard.press('Escape');
	await sliceInUi(page);
	expect(configBlock(await downloadGcode(page)).wall_loops).toBe('5');
});

test('envoi vers Moonraker : le fichier arrive sur l’imprimante @F:core.moonraker', async ({ page }) => {
	const moonraker = process.env.MOONRAKER_URL;
	test.skip(!moonraker, 'MOONRAKER_URL non défini (stack.sh démarre un Moonraker virtuel via Docker)');
	const list = async () => {
		const r = await page.request.get(`${moonraker}/server/files/list?root=gcodes`);
		return ((await r.json()).result as { path: string }[]).map((f) => f.path);
	};
	const projectId = await bootstrap(page);
	await page.goto('/printers');
	await page.getByTestId('printer-add').click();
	await page.getByTestId('printer-moonraker-url').fill(moonraker!);
	await page.getByTestId('printer-save').click();
	// bootstrap a déjà déclaré une imprimante : attendre la ligne de la nouvelle.
	await expect(page.getByTestId('printer-row').filter({ hasText: moonraker! })).toBeVisible();
	const before = new Set(await list());
	await page.goto(`/projects/${projectId}`);
	await importModel(page, MODEL);
	await sliceInUi(page);
	await page.getByTestId('send-to-printer').click();
	await page.getByTestId('send-printer-select').selectOption({ index: 0 });
	await page.getByTestId('send-confirm').click();
	await expect(page.getByTestId('send-success')).toBeVisible({ timeout: 60_000 });
	await expect.poll(async () => (await list()).filter((p) => !before.has(p) && p.endsWith('.gcode')).length).toBeGreaterThan(0);
});
