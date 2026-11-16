// Régressions d'ergonomie (lot ux-audit) : menus fermés par Échap, focus des
// dialogues modaux, titres d'onglet, libellés FR des réglages par objet,
// clés surchargeables (Orca SettingsFactory), Tab, nom de projet, envoi
// Moonraker depuis le dialogue, nom du G-code exporté.
import { expect, test } from '@playwright/test';
import { bootstrap, importModel, sliceInUi } from '../lib/app';

test.use({ locale: 'fr-FR' });

test('Échap referme le menu contextuel objet et le menu Fichier', async ({ page }) => {
	await bootstrap(page);
	await importModel(page, 'cube20');
	await page.getByTestId('object-row').first().click({ button: 'right' });
	await expect(page.getByRole('menu')).toHaveCount(1);
	await page.keyboard.press('Escape');
	await expect(page.getByRole('menu')).toHaveCount(0);
	await page.getByRole('button', { name: 'Fichier', exact: true }).click();
	await expect(page.getByRole('menu')).toHaveCount(1);
	await page.keyboard.press('Escape');
	await expect(page.getByRole('menu')).toHaveCount(0);
});

test('un dialogue modal prend le focus : Suppr ne supprime pas l’objet derrière', async ({ page }) => {
	await bootstrap(page);
	await importModel(page, 'cube20');
	await page.getByTestId('object-row').first().click();
	await page.getByTestId('open-settings-print').click();
	const dialog = page.getByTestId('settings-dialog');
	await expect(dialog).toBeVisible();
	await expect.poll(() => page.evaluate(() => !!document.activeElement?.closest('[data-testid=settings-dialog]'))).toBe(true);
	await page.keyboard.press('Delete');
	await expect(page.getByTestId('object-row')).toHaveCount(1);
	await page.keyboard.press('Escape');
	await expect(dialog).toHaveCount(0);
	await expect(page.getByTestId('open-settings-print')).toBeFocused();
});

test('titres d’onglet et libellés FR du panneau « + Réglage »', async ({ page }) => {
	await bootstrap(page);
	await expect(page).toHaveTitle(/— Web-Slicer$/);
	await importModel(page, 'cube20');
	await page.getByTestId('object-row').first().click();
	await page.getByTestId('object-settings-add').click();
	await page.getByTestId('object-settings-search').fill('paroi');
	await expect(page.getByTestId('object-settings-results')).toContainText('Ratio de débit de la paroi extérieure');
	await page.goto('/library');
	await expect(page).toHaveTitle('Récent — Web-Slicer');
});

test('« + Réglage » : ni limites machine ni catégorie Extruders avec un filament (Orca SettingsFactory)', async ({ page }) => {
	await bootstrap(page);
	await importModel(page, 'cube20');
	await page.getByTestId('object-row').first().click();
	await page.getByTestId('object-settings-add').click();
	const results = page.getByTestId('object-settings-results');
	await page.getByTestId('object-settings-search').fill('machine_max');
	await expect(results.getByTestId('object-settings-result')).toHaveCount(0);
	await page.getByTestId('object-settings-search').fill('inner_wall_filament_id');
	await expect(results.getByTestId('object-settings-result')).toHaveCount(0);
	await page.getByTestId('object-settings-search').fill('wall_loops');
	await expect(results.locator('[data-key="wall_loops"]')).toHaveCount(1);
});

test('Tab : bascule Préparer/Aperçu depuis la scène, navigation native depuis un bouton', async ({ page }) => {
	await bootstrap(page);
	await importModel(page, 'cube20');
	const importBtn = page.getByRole('button', { name: 'Importer', exact: true });
	await importBtn.focus();
	await page.keyboard.press('Tab');
	await expect(page.getByTestId('scene-canvas')).toBeVisible();
	await expect(importBtn).not.toBeFocused();
	await page.locator('body').click({ position: { x: 5, y: 5 } });
	await page.locator('body').focus();
	await page.keyboard.press('Tab');
	await expect(page.getByTestId('scene-canvas')).toBeHidden();
});

test('nom du projet : « Sans titre » devient le nom du premier modèle importé (Orca « Untitled »)', async ({ page }) => {
	await bootstrap(page);
	const res = await page.request.post('/api/projects', { data: { name: 'Sans titre' } });
	expect(res.ok()).toBe(true);
	const { id } = (await res.json()) as { id: string };
	await page.goto(`/projects/${id}`);
	await expect(page.getByTestId('scene-canvas')).toBeVisible();
	await importModel(page, 'cube20');
	await expect(page).toHaveTitle('cube20 — Web-Slicer');
	await expect.poll(async () => ((await (await page.request.get(`/api/projects/${id}`)).json()) as { name: string }).name).toBe('cube20');
	// Un nom choisi par l'utilisateur n'est plus remplacé.
	await importModel(page, 'cylinder');
	await expect(page).toHaveTitle('cube20 — Web-Slicer');
});

test('envoi : imprimante non connectée listée, URL Moonraker saisie dans le dialogue', async ({ page }) => {
	const moonraker = process.env.MOONRAKER_URL ?? 'http://127.0.0.1:17125';
	const projectId = await bootstrap(page);
	const presets = (await (await page.request.get('/api/presets?kind=machine')).json()) as { id: string; name: string }[];
	const machine = presets.find((p) => /Voron 2\.4 350 0\.4/.test(p.name)) ?? presets[0];
	const created = await page.request.post('/api/printers', { data: { name: 'Voron atelier', machine_preset_id: machine.id } });
	expect(created.ok()).toBe(true);
	await page.goto(`/projects/${projectId}`);
	await importModel(page, 'cube20');
	await sliceInUi(page);
	await page.getByTestId('send-to-printer').click();
	const select = page.getByTestId('send-printer-select');
	await expect(select).toContainText('Voron atelier — non connectée');
	await select.selectOption({ label: 'Voron atelier — non connectée' });
	await expect(page.getByTestId('send-confirm')).toBeDisabled();
	await page.getByTestId('send-moonraker-url').fill(moonraker.replace(/^http:\/\//, ''));
	await page.getByTestId('send-save-url').click();
	await expect(select).toContainText(`Voron atelier — ${moonraker}`);
	await expect(page.getByTestId('send-confirm')).toBeEnabled();
	await page.getByTestId('send-confirm').click();
	await expect(page.getByTestId('send-success')).toBeVisible({ timeout: 60_000 });
});

test('export G-code : nommé comme Orca ({input_filename_base}_{filament_type}_{print_time}.gcode)', async ({ page }) => {
	await bootstrap(page);
	await importModel(page, 'cube20');
	await sliceInUi(page);
	const [dl] = await Promise.all([page.waitForEvent('download'), page.getByTestId('export-gcode-button').click()]);
	expect(dl.suggestedFilename()).toMatch(/^Projet harness [0-9a-f]+_PLA_(\d+d)?(\d+h)?(\d+m)?(\d+s)?\.gcode$/);
});
