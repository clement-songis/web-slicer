// Phase 19 (parité Orca) : historique d'annulation en liste, création de
// filament / d'imprimante personnalisés, historique de calibration PA.
import { expect, test, type Page } from '@playwright/test';
import { bootstrap, importModel } from '../lib/app';

const rows = (page: Page) => page.getByTestId('object-row');

test('historique d annulation : liste nommée, plusieurs étapes d un coup @F:ui.undo-history', async ({ page }) => {
	await bootstrap(page);
	const undo = page.getByTestId('history-undo');
	// Une étape par import (historique différé de 400 ms après la fin de l'import).
	await importModel(page, 'cube20');
	await expect(undo).toHaveAttribute('aria-label', 'Annuler : Ajouter « cube20.stl »', { timeout: 60_000 });
	await importModel(page, 'cylinder');
	await expect(undo).toHaveAttribute('aria-label', 'Annuler : Ajouter « cylinder.stl »', { timeout: 60_000 });
	await page.getByRole('button', { name: 'Objets', exact: true }).click();
	await expect(rows(page)).toHaveCount(2);
	await page.getByRole('button', { name: 'Supprimer cylinder.stl' }).click();
	await expect(rows(page)).toHaveCount(1);

	await page.getByTestId('history-undo-list').click();
	const steps = page.getByTestId('history-menu').getByTestId('history-step');
	await expect(steps.first()).toContainText('Supprimer « cylinder.stl »');
	expect(await steps.count()).toBeGreaterThanOrEqual(2);
	await steps.first().click();
	await expect(rows(page)).toHaveCount(2);

	// Rétablir depuis la liste.
	await page.getByTestId('history-redo-list').click();
	await expect(page.getByTestId('history-menu').getByTestId('history-step').first()).toContainText('Supprimer « cylinder.stl »');
	await page.getByTestId('history-menu').getByTestId('history-step').first().click();
	await expect(rows(page)).toHaveCount(1);

	// Plusieurs étapes : tout annuler jusqu'à l'ajout du cube.
	await page.getByTestId('history-undo-list').click();
	const all = page.getByTestId('history-menu').getByTestId('history-step');
	const n = await all.count();
	await all.nth(n - 1).click();
	await expect(rows(page)).toHaveCount(0);
});

test('filament personnalisé : fournisseur, type, série, hérite du filament courant @F:ui.create-filament', async ({ page }) => {
	await bootstrap(page);
	await page.getByTestId('create-filament').click();
	const dlg = page.getByTestId('create-filament-dialog');
	await dlg.getByTestId('create-filament-vendor').fill('Atelier');
	await dlg.getByTestId('create-filament-type').selectOption('PETG');
	await dlg.getByTestId('create-filament-serial').fill('Mat');
	await expect(dlg.getByTestId('create-filament-name')).toHaveText('Atelier PETG Mat');
	await dlg.getByTestId('create-filament-submit').click();
	await expect(dlg).toHaveCount(0);
	const list = (await (await page.request.get('/api/presets?kind=filament')).json()) as { id: string; name: string }[];
	const created = list.find((p) => p.name === 'Atelier PETG Mat')!;
	expect(created).toBeTruthy();
	const detail = await (await page.request.get(`/api/presets/${created.id}`)).json();
	expect(detail.inherits).toMatch(/^Generic PETG/);
	expect(detail.values.filament_type).toEqual(['PETG']);
	await expect(page.locator('[data-testid="filament-slot"] select').first()).toHaveValue(created.id);
	// Doublon refusé.
	await page.getByTestId('create-filament').click();
	await dlg.getByTestId('create-filament-vendor').fill('Atelier');
	await dlg.getByTestId('create-filament-type').selectOption('PETG');
	await dlg.getByTestId('create-filament-serial').fill('Mat');
	await dlg.getByTestId('create-filament-submit').click();
	await expect(dlg.getByRole('alert')).toContainText('existe déjà');
});

test('imprimante personnalisée : volume, buse, modèle de départ ; process compatibles @F:ui.create-printer', async ({ page }) => {
	await bootstrap(page);
	await page.goto('/printers');
	await page.getByTestId('printer-create-custom').click();
	const dlg = page.getByTestId('create-printer-dialog');
	await dlg.getByTestId('create-printer-vendor').fill('Atelier');
	await dlg.getByTestId('create-printer-model').fill('Cube 300');
	await dlg.getByTestId('create-printer-width').fill('300');
	await dlg.getByTestId('create-printer-depth').fill('250');
	await dlg.getByTestId('create-printer-height').fill('280');
	await dlg.getByTestId('create-printer-nozzle').selectOption('0.6');
	await expect(dlg.getByTestId('create-printer-name')).toHaveText('Atelier Cube 300 0.6 nozzle');
	await dlg.getByTestId('create-printer-submit').click();
	await expect(dlg).toHaveCount(0);
	await expect(page.getByTestId('printer-row').filter({ hasText: 'Atelier Cube 300' })).toBeVisible();
	const machines = (await (await page.request.get('/api/presets?kind=machine')).json()) as { id: string; name: string }[];
	const m = machines.find((p) => p.name === 'Atelier Cube 300 0.6 nozzle')!;
	const detail = await (await page.request.get(`/api/presets/${m.id}`)).json();
	expect(detail.values.printable_area).toEqual(['0x0', '300x0', '300x250', '0x250']);
	expect(detail.values.printable_height).toBe(280);
});

// Backend (phase19/BACKEND.md §1) : un preset machine utilisateur n'hérite pas
// encore de la compatibilité de son parent → aucun process proposé.
test('imprimante personnalisée : process et filaments compatibles proposés @F:ui.create-printer-compat', async ({ page }) => {
	await bootstrap(page);
	const machines = (await (await page.request.get('/api/presets?kind=machine')).json()) as { name: string; origin: string }[];
	const base = machines.find((p) => p.name === 'Voron 2.4 350 0.4 nozzle')!;
	const created = await (
		await page.request.post('/api/presets', {
			data: { kind: 'machine', name: 'Atelier Compat 0.4 nozzle', inherits: base.name, values: { printable_height: 200 } }
		})
	).json();
	for (const kind of ['process', 'filament']) {
		const list = await (await page.request.get(`/api/presets?kind=${kind}&printer=${encodeURIComponent(created.name)}`)).json();
		expect((list as unknown[]).length, kind).toBeGreaterThan(0);
	}
});

test('historique Pressure Advance : enregistrer, appliquer au filament, persistance @F:ui.pa-history', async ({ page }) => {
	await bootstrap(page);
	const openHistory = async () => {
		await page.getByRole('button', { name: /^Calibration$/ }).first().click();
		await page.getByRole('menuitem', { name: /Pressure advance history|Historique Pressure advance/ }).click();
	};
	await openHistory();
	const dlg = page.getByTestId('pa-history-dialog');
	await expect(dlg.getByTestId('pa-empty')).toBeVisible();
	await dlg.getByTestId('pa-k').fill('0,045');
	await dlg.getByTestId('pa-add').click();
	await expect(dlg.getByTestId('pa-row')).toHaveCount(1);
	await expect(dlg.getByTestId('pa-row')).toContainText('0.045');
	await dlg.getByTestId('pa-apply').click();
	await expect(dlg).toHaveCount(0);
	const settings = page.getByTestId('settings-dialog');
	await expect(settings).toBeVisible();
	await settings.getByTestId('settings-mode-develop').click();
	await settings.getByTestId('settings-tab-filament').click();
	const row = settings.locator('[data-param=pressure_advance]');
	await expect(row.locator('input').last()).toHaveValue('0.045');
	await page.keyboard.press('Escape');
	await page.reload();
	await expect(page.getByTestId('scene-canvas')).toBeVisible();
	await openHistory();
	await expect(dlg.getByTestId('pa-row')).toHaveCount(1);
	await dlg.getByTestId('pa-delete').click();
	await expect(dlg.getByTestId('pa-empty')).toBeVisible();
});
