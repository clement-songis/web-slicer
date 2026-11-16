// Aperçu après tranchage réel (pile complète) : deux plateaux tranchés
// séparément, curseur de couches (haute / basse / mode une couche) vérifié sur
// le nombre de segments réellement rendus (`data-visible-segments` de
// `preview-canvas`), coloration par filament, export G-code du plateau 2.
import { expect, test, type Page } from '@playwright/test';
import { bootstrap, downloadGcode, importModel } from '../lib/app';

/** Tranche le plateau actif et attend la méta d'aperçu **de ce tranchage**
 * (l'attribut `data-layer-count` peut encore porter le tranchage précédent). */
async function sliceActivePlate(page: Page): Promise<number> {
	const [meta] = await Promise.all([
		page.waitForResponse((r) => /\/api\/gcodes\/[^/]+\/preview\/meta/.test(r.url()) && r.ok(), {
			timeout: 170_000
		}),
		page.getByTestId('slice-button').click()
	]);
	const layers = ((await meta.json()) as { layer_count: number }).layer_count;
	await expect(page.getByTestId('preview-view')).toHaveAttribute('data-layer-count', String(layers), {
		timeout: 30_000
	});
	return layers;
}

function canvas(page: Page) {
	return page.getByTestId('preview-canvas');
}

async function segments(page: Page): Promise<number> {
	return Number(await canvas(page).getAttribute('data-visible-segments'));
}

async function showPrepare(page: Page) {
	await page.getByRole('navigation', { name: "Vues de l'éditeur" }).getByRole('button', { name: /prepare|préparer/i }).click();
}

async function selectPlate(page: Page, index: number) {
	// La barre de plateaux vit dans l'onglet « Objets » du panneau latéral.
	await page.getByRole('button', { name: 'Objets', exact: true }).click();
	await page.getByTestId('plate-tab').nth(index).getByRole('button').first().click();
	await expect(page.getByTestId('plate-tab').nth(index)).toHaveAttribute('data-active', 'true');
}

test('aperçu : deux plateaux, curseur de couches, coloration filament, export du plateau 2 @F:editor.preview-plates', async ({ page }) => {
	test.setTimeout(420_000);
	await bootstrap(page);

	// Plateau 1 : cube de 20 mm ; plateau 2 : deux pavés de 10 mm de haut.
	await importModel(page, 'cube20');
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
	await page.getByTestId('toolbar-addplate').click();
	await page.getByRole('button', { name: 'Objets', exact: true }).click();
	await expect(page.getByTestId('plate-tab')).toHaveCount(2);
	await selectPlate(page, 1);
	await importModel(page, 'twoparts');
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
	await expect(page.getByTestId('plate-tab').nth(0)).toContainText('(1)');
	await expect(page.getByTestId('plate-tab').nth(1)).toContainText('(1)');

	// — Plateau 1 —
	await selectPlate(page, 0);
	const layers1 = await sliceActivePlate(page);
	await expect(canvas(page)).toBeVisible();
	const full = await segments(page);
	expect(full, 'segments rendus, plage complète').toBeGreaterThan(0);
	await expect(canvas(page)).toHaveAttribute('data-layer-high', String(layers1 - 1));

	// Poignée haute à mi-hauteur : moins de segments.
	const mid = Math.floor(layers1 / 2);
	await page.getByTestId('layer-slider').fill(String(mid));
	await expect(canvas(page)).toHaveAttribute('data-layer-high', String(mid));
	await expect.poll(() => segments(page)).toBeLessThan(full);
	const upToMid = await segments(page);
	expect(upToMid).toBeGreaterThan(0);

	// Poignée basse relevée : plage [low, mid] encore plus courte.
	const low = Math.floor(mid / 2);
	await page.getByLabel('Couche basse').fill(String(low));
	await expect(canvas(page)).toHaveAttribute('data-layer-low', String(low));
	await expect.poll(() => segments(page)).toBeLessThan(upToMid);
	const window = await segments(page);

	// Mode une couche : seule la couche haute est rendue.
	await page.getByRole('button', { name: '1 couche', exact: true }).click();
	await expect(canvas(page)).toHaveAttribute('data-layer-low', String(mid));
	await expect.poll(() => segments(page)).toBeLessThan(window);
	expect(await segments(page)).toBeGreaterThan(0);
	await page.getByRole('button', { name: '1 couche', exact: true }).click();

	// Coloration par filament : un seul filament → une seule couleur rendue ;
	// la coloration par type en produit plusieurs.
	const select = page.getByLabel("Coloration de l'aperçu");
	await select.selectOption('type');
	await expect(canvas(page)).toHaveAttribute('data-coloration', 'type');
	const typeColors = ((await canvas(page).getAttribute('data-colors')) ?? '').split(' ').filter(Boolean);
	expect(typeColors.length, 'couleurs par type').toBeGreaterThan(1);
	await select.selectOption('filament');
	await expect(canvas(page)).toHaveAttribute('data-coloration', 'filament');
	await expect(canvas(page)).toHaveAttribute('data-colors', /^#[0-9a-f]{6}$/);

	// — Plateau 2 —
	await showPrepare(page);
	await selectPlate(page, 1);
	const layers2 = await sliceActivePlate(page);
	expect(layers2, 'plateau 2 (10 mm) moins haut que le plateau 1 (20 mm)').toBeLessThan(layers1);
	await expect(canvas(page)).toHaveAttribute('data-layer-high', String(layers2 - 1));

	// Export : le G-code téléchargé est celui du plateau 2.
	const gcode = await downloadGcode(page);
	const layerChanges = gcode.split('\n').filter((l) => l.startsWith(';LAYER_CHANGE')).length;
	expect(layerChanges).toBe(layers2);
	const zs = [...gcode.matchAll(/^;Z:([\d.]+)/gm)].map((m) => Number(m[1]));
	expect(Math.max(...zs)).toBeCloseTo(10, 0);
});
