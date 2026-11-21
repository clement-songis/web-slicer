// Vue/aperçu (parité Orca côté web) : fenêtre G-code synchronisée avec la
// couche, export des trajets en OBJ, étiquettes et surplombs de la scène.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { expect, test, type Page } from '@playwright/test';
import { bootstrap, importModel } from '../lib/app';

const FILE = /^(file|fichier)$/i;
const VIEW = /^(view|vue|affichage)$/i;
const GCODE = fileURLToPath(new URL('../../../engine/tests/fixtures/cube_bbl.gcode', import.meta.url));

async function menu(page: Page, top: RegExp, item: RegExp, sub?: RegExp) {
	await page.getByRole('button', { name: top }).first().click();
	if (sub) await page.getByRole('menu').getByRole('button', { name: sub }).first().click();
	await page.getByRole('menuitem', { name: item }).first().click();
}

async function openGcode(page: Page) {
	const [chooser] = await Promise.all([
		page.waitForEvent('filechooser'),
		menu(page, FILE, /open g-code|ouvrir.*g-code/i)
	]);
	await chooser.setFiles(GCODE);
	await expect(page.getByTestId('preview-canvas')).toBeVisible({ timeout: 30_000 });
}

test('fenêtre G-code synchronisée avec la couche @F:editor.view-gcode-window', async ({ page }) => {
	await bootstrap(page);
	await openGcode(page);
	await menu(page, VIEW, /g-code window|fenêtre g-code/i);
	const panel = page.getByTestId('gcode-window');
	await expect(panel).toBeVisible();
	const high = await page.getByTestId('preview-canvas').getAttribute('data-layer-high');
	await expect(panel).toHaveAttribute('data-layer', high as string);
	// Première couche : ligne 651 du fichier de référence (« ; CHANGE_LAYER »).
	await page.getByTestId('layer-slider').fill('0');
	await expect(panel).toHaveAttribute('data-layer', '0');
	await expect(panel).toHaveAttribute('data-current-line', '651');
	await expect(panel.locator('.bg-primary\\/30')).toContainText('; CHANGE_LAYER');
	await page.getByTestId('layer-slider').fill('5');
	await expect(panel).toHaveAttribute('data-layer', '5');
	const line = Number(await panel.getAttribute('data-current-line'));
	expect(line).toBeGreaterThan(651);
	const lines = readFileSync(GCODE, 'utf8').split(/\r?\n/);
	expect(lines[line - 1].trim()).toBe('; CHANGE_LAYER');
});

test('export des trajets en OBJ @F:editor.export-toolpaths-obj', async ({ page }) => {
	await bootstrap(page);
	await openGcode(page);
	const [dl] = await Promise.all([
		page.waitForEvent('download'),
		menu(page, FILE, /toolpaths|trajectoires/i, /^(export|exporter)\b/i)
	]);
	expect(dl.suggestedFilename()).toMatch(/\.obj$/);
	const text = readFileSync((await dl.path()) as string, 'utf8');
	const vertices = (text.match(/^v /gm) ?? []).length;
	expect(vertices).toBeGreaterThan(100);
	expect((text.match(/^l /gm) ?? []).length).toBe(vertices / 2);
});

test('étiquettes et surplombs de la scène @F:editor.view-labels-overhang', async ({ page }) => {
	await bootstrap(page);
	await importModel(page, 'cube20');
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
	const scene = page.getByTestId('scene-canvas');

	await menu(page, VIEW, /show labels|étiquettes/i);
	await expect(scene).toHaveAttribute('data-labels', '1');
	await expect(page.getByTestId('object-label')).toContainText('cube20');

	await menu(page, VIEW, /show overhang|surplomb/i);
	// Posé sur le plateau : le dessous est exclu (comme Orca) → 0 facette.
	await expect(scene).toHaveAttribute('data-overhang-faces', '0');
	// Relevé à z = 30 : le dessous (2 triangles) devient un surplomb.
	await page.getByTestId('object-row').first().click();
	await page
		.getByRole('toolbar', { name: 'Outils' })
		.getByRole('button', { name: /flatten|poser/i })
		.filter({ visible: true })
		.first()
		.click();
	await page.getByLabel(/Position \(mm\) z/i).fill('30');
	await expect(scene).toHaveAttribute('data-overhang-faces', '2');
	await menu(page, VIEW, /show overhang|surplomb/i);
	await expect(scene).not.toHaveAttribute('data-overhang-faces', /.*/);
});
