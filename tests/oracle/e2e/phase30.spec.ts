// Phase 30 — plateaux côte à côte dans la vue 3D (PartPlateList d'Orca : clic
// pour activer, glisser un objet d'un plateau à l'autre), barre de messages
// sans décalage de la mise en page, indication « Glissez un modèle » masquée
// dès qu'un objet existe (même en échec de conversion).
import { expect, test, type Page } from '@playwright/test';
import { writeFileSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { bootstrap, importModel } from '../lib/app';

test.use({ locale: 'fr-FR' });

const hint = (page: Page) => page.getByText('Glissez un modèle ici');

test('plateaux côte à côte : actif mis en évidence, clic sur un autre plateau pour l’activer @F:toolbar.addplate', async ({ page }) => {
	test.setTimeout(180_000);
	await bootstrap(page);
	await importModel(page, 'cube20');
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
	await page.getByRole('button', { name: 'Objets', exact: true }).click();
	const canvas = page.getByTestId('scene-canvas');
	// Plateau unique : pas d'étiquette, pas de voisin.
	await expect(canvas).toHaveAttribute('data-plates', '1');
	await expect(page.getByTestId('scene-plate-label')).toHaveCount(0);
	await page.getByRole('button', { name: 'Ajouter un plateau' }).first().click();
	await expect(canvas).toHaveAttribute('data-plates', '2');
	// Grille d'Orca : 2 plateaux → 2 colonnes, pas = largeur × 1,2.
	await expect(canvas).toHaveAttribute('data-plate-offsets', /^1:\d+(\.\d)?,0\.0$/);
	const labels = page.getByTestId('scene-plate-label');
	await expect(labels).toHaveCount(2);
	await expect(page.locator('[data-testid=scene-plate-label][data-active=true]')).toHaveAttribute(
		'data-plate-index',
		'1'
	);
	// Le cube reste sur le plateau 1 : le plateau 2 est vide.
	await expect(page.locator('[data-testid=scene-plate-label][data-active=false]')).toHaveText('Plateau 2');
	await page.locator('[data-testid=scene-plate-label][data-active=false]').click();
	await expect(page.locator('[data-testid=plate-tab][data-active=true]')).toContainText('Plateau 2');
	await expect(page.locator('[data-testid=scene-plate-label][data-active=true]')).toHaveAttribute(
		'data-plate-index',
		'2'
	);
	// Le plateau 1 (désormais voisin, à gauche) affiche son objet : « Plateau 1 · 1 » ;
	// dézoom (O) pour le voir, puis clic sur son étiquette pour revenir.
	await page.keyboard.press('o');
	await page.keyboard.press('o');
	await expect(page.locator('[data-testid=scene-plate-label][data-active=false]')).toHaveText(
		'Plateau 1 · 1'
	);
	await expect(canvas).toHaveAttribute('data-plate-offsets', /^0:-\d+(\.\d)?,0\.0$/);
	// Le plateau actif est vide : indication de dépôt affichée.
	await expect(hint(page)).toBeVisible();
	await page.locator('[data-testid=scene-plate-label][data-active=false]').click();
	await expect(page.locator('[data-testid=plate-tab][data-active=true]')).toContainText('Plateau 1');
	await expect(hint(page)).toHaveCount(0);
});

test('glisser un objet sur un autre plateau le lui affecte (repère du plateau d’arrivée) @F:toolbar.addplate', async ({ page }) => {
	test.setTimeout(180_000);
	await bootstrap(page);
	await importModel(page, 'cube20');
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
	await page.getByRole('button', { name: 'Objets', exact: true }).click();
	await page.getByRole('button', { name: 'Ajouter un plateau' }).first().click();
	const canvas = page.getByTestId('scene-canvas');
	await expect(canvas).toHaveAttribute('data-plates', '2');
	// Vue de dessus, dézoomée : les deux plateaux sont à l'écran.
	await canvas.click({ position: { x: 5, y: 5 } });
	await page.keyboard.press('Control+1');
	await expect(canvas).toHaveAttribute('data-camera-view', 'top');
	await page.keyboard.press('o');
	await page.keyboard.press('o');
	await page.getByTestId('object-row').filter({ hasText: 'cube20.stl' }).getByText('cube20.stl').click();
	await expect(canvas).toHaveAttribute('data-object-anchor', /^\d+,\d+$/);
	await page.waitForTimeout(500);
	const [ax, ay] = (await canvas.getAttribute('data-object-anchor'))!.split(',').map(Number);
	const box = (await canvas.boundingBox())!;
	const target = (await page.locator('[data-testid=scene-plate-label][data-active=false]').boundingBox())!;
	// Étiquette = milieu du bord arrière du plateau 2 (haut en vue de dessus) :
	// 80 px plus bas, on est à l'intérieur du plateau 2.
	const tx = target.x + target.width / 2;
	const ty = target.y + 80;
	await page.mouse.move(box.x + ax, box.y + ay);
	await page.mouse.down();
	await page.mouse.move(tx, ty, { steps: 20 });
	await page.mouse.up();
	await expect(page.getByTestId('editor-message')).toContainText('« cube20.stl » déplacé sur Plateau 2');
	// Plateau actif inchangé (vue stable) ; l'objet est listé sur le plateau 2.
	await expect(page.locator('[data-testid=plate-tab][data-active=true]')).toContainText('Plateau 1');
	await expect(page.locator('[data-testid=scene-plate-label][data-active=false]')).toHaveText(
		'Plateau 2 · 1'
	);
	await expect(page.locator('[data-testid=plate-tab]').nth(1)).toContainText('(1)');
	await expect(hint(page)).toBeVisible();
	// Activé, le plateau 2 montre l'objet dans son propre repère (sur le plateau).
	await page.locator('[data-testid=plate-tab]').nth(1).getByRole('button').first().click();
	const positions = await canvas.getAttribute('data-object-positions');
	const [x, y] = positions!.split(':')[1].split(',').map(Number);
	expect(x).toBeGreaterThan(0);
	expect(y).toBeGreaterThan(0);
	expect(x).toBeLessThan(400);
	expect(y).toBeLessThan(400);
});

test('barre de messages : aucun décalage, estompée ensuite, transparente aux clics @F:toolbar.arrange', async ({ page }) => {
	test.setTimeout(120_000);
	await bootstrap(page);
	const aside = page.getByRole('complementary', { name: 'Configuration' });
	const before = (await aside.boundingBox())!;
	await expect(page.getByTestId('editor-message')).toHaveCount(0);
	await importModel(page, 'cube20');
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
	await page.getByTestId('toolbar-arrange').click();
	await page.getByTestId('arrange-all').click();
	const msg = page.getByTestId('editor-message');
	await expect(msg).toContainText('Arrangement', { timeout: 30_000 });
	const after = (await aside.boundingBox())!;
	expect(after.y).toBe(before.y);
	expect(after.height).toBe(before.height);
	await expect(msg).toHaveCSS('pointer-events', 'none');
	// Estompée après quelques secondes (le texte reste lisible dans le DOM).
	await expect(msg).toHaveAttribute('data-faded', 'true', { timeout: 10_000 });
	await expect(msg).toContainText('Arrangement');
});

test('modèle en échec de conversion listé : l’indication « Glissez un modèle » disparaît @F:core.import', async ({ page }) => {
	test.setTimeout(120_000);
	await bootstrap(page);
	await expect(hint(page)).toBeVisible();
	const dir = mkdtempSync(join(tmpdir(), 'p30-'));
	const bad = join(dir, 'casse.stl');
	writeFileSync(bad, 'solid casse\nfacet normal 0 0 1\n  outer loop\n');
	await page.getByTestId('import-input').setInputFiles(bad);
	await expect(page.getByRole('alert').filter({ hasText: 'casse.stl' })).toContainText(
		'Le chargement du fichier modèle a échoué',
		{ timeout: 60_000 }
	);
	// Import neuf en échec : l'objet est retiré (rien sur le plateau), l'indication reste.
	await expect(hint(page)).toBeVisible();
	// Projet rouvert : le modèle en échec reste listé avec sa cause → plus d'indication.
	await page.reload();
	await expect(page.getByTestId('object-row').filter({ hasText: 'casse.stl' })).toBeVisible({
		timeout: 60_000
	});
	await expect(hint(page)).toHaveCount(0);
});
