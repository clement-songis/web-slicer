// Gestion des presets et G-code personnalisé (parité Orca Tab / DiffPresetDialog /
// UnsavedChangesDialog / EditGCodeDialog) : marqueurs « modifié » et « différent
// du système » par option avec retour en arrière, renommer / supprimer un preset
// utilisateur, comparer deux presets, transférer des modifications non
// enregistrées, éditeur de G-code avec variables.
import { expect, test } from '@playwright/test';
import { bootstrap, openParam, paramRow } from '../lib/app';

test('presets : marqueurs par option, renommer, comparer, transférer, supprimer @F:ui.preset-management', async ({ page }) => {
	test.setTimeout(240_000);
	const id = await bootstrap(page);
	const systemId = ((await (await page.request.get(`/api/projects/${id}`)).json()) as { active_presets: { process: string } })
		.active_presets.process;
	const dialog = page.getByTestId('settings-dialog');
	let control = await openParam(page, 'print', 'Strength', 'wall_loops');
	const row = dialog.locator('[data-param="wall_loops"]');
	await expect(control).not.toHaveValue('');
	const original = await control.inputValue();

	// Modifié → marqueur ; retour à la valeur enregistrée.
	await control.fill('5');
	await control.press('Tab');
	await expect(row).toHaveAttribute('data-modified', 'true');
	await row.getByTestId('option-reset-saved').click();
	await expect(control).toHaveValue(original);
	await expect(row).not.toHaveAttribute('data-modified', /.+/);

	// Copie utilisateur (enregistrer un preset système modifié).
	await control.fill('5');
	await control.press('Tab');
	await page.getByTestId('settings-apply').click();
	await expect(dialog).toHaveCount(0);
	control = await openParam(page, 'print', 'Strength', 'wall_loops');
	await expect(row).toHaveAttribute('data-from-system', 'true');
	await expect(dialog.getByTestId('preset-bar')).toContainText('Utilisateur');

	// Renommer.
	await dialog.getByTestId('preset-rename').click();
	await dialog.getByTestId('preset-rename-input').fill('Mon process 5 parois');
	await dialog.getByTestId('preset-rename-input').press('Enter');
	await expect(dialog.getByTestId('preset-name')).toHaveText('Mon process 5 parois');

	// Comparer avec le preset système parent.
	await dialog.getByTestId('preset-compare').click();
	const cmp = page.getByTestId('compare-dialog');
	await expect(cmp.locator('[data-testid=compare-row][data-key=wall_loops]')).toBeVisible();
	await page.keyboard.press('Escape');
	await expect(cmp).toHaveCount(0);

	// Modification non enregistrée puis changement de preset : transférer.
	await control.fill('7');
	await control.press('Tab');
	await page.getByRole('button', { name: /^(Fermer|Close)$/ }).first().click();
	await page.getByTestId('preset-select-print').selectOption(systemId);
	const unsaved = page.getByTestId('unsaved-preset-dialog');
	await expect(unsaved.locator('[data-testid=unsaved-row][data-key=wall_loops]')).toBeVisible();
	await unsaved.getByTestId('unsaved-transfer').click();
	await expect(page.getByTestId('preset-select-print')).toHaveValue(systemId);
	control = await openParam(page, 'print', 'Strength', 'wall_loops');
	await expect(control).toHaveValue('7');
	await row.getByTestId('option-reset-saved').click();
	await page.getByRole('button', { name: /^(Fermer|Close)$/ }).first().click();

	// Retour au preset utilisateur puis suppression → preset système sélectionné.
	const userId = ((await (await page.request.get('/api/presets?kind=process')).json()) as { id: string; name: string }[]).find(
		(p) => p.name === 'Mon process 5 parois'
	)!.id;
	await page.getByTestId('preset-select-print').selectOption(userId);
	// Brouillon encore différent du preset système (transfert) : abandonner.
	const discard = page.getByTestId('unsaved-preset-dialog').getByTestId('unsaved-discard');
	await page.waitForTimeout(500);
	if (await discard.isVisible().catch(() => false)) await discard.click();
	await expect(page.getByTestId('preset-select-print')).toHaveValue(userId);
	await page.getByTestId('open-settings-print').click();
	await expect(dialog.getByTestId('preset-name')).toHaveText('Mon process 5 parois');
	await dialog.getByTestId('preset-delete').click();
	await dialog.getByTestId('preset-delete-confirm').click();
	await expect(page.getByTestId('preset-select-print')).toHaveValue(systemId);
	await expect.poll(async () => (await page.request.get(`/api/presets/${userId}`)).status()).toBe(404);
	await expect(page.getByTestId('editor-message')).toContainText('supprimé');
});

test('G-code personnalisé : éditeur avec variables, insertion au curseur @F:ui.gcode-editor', async ({ page }) => {
	await bootstrap(page);
	const row = await paramRow(page, 'printer', 'Machine G-code', 'layer_change_gcode');
	await expect(row).toBeVisible();
	await row.getByTestId('gcode-edit').click();
	const ed = page.getByTestId('gcode-editor');
	const area = ed.getByTestId('gcode-editor-text');
	await area.fill('; couche\n');
	await ed.getByTestId('gcode-placeholder-search').fill('layer_z');
	const item = ed.locator('[data-testid=gcode-placeholder][data-key=layer_z]').first();
	await item.click();
	await expect(ed.getByTestId('gcode-placeholder-help')).toContainText(/Layer Z|Z/);
	await item.dblclick();
	await expect(area).toHaveValue('; couche\n{layer_z}');
	await ed.getByTestId('gcode-editor-apply').click();
	await expect(row.locator('textarea')).toHaveValue('; couche\n{layer_z}');
	await expect(row).toHaveAttribute('data-modified', 'true');
});
