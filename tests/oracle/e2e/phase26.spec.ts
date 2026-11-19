// Phase 26 — deuxième audit d'utilisation réelle : texte/SVG en relief posé sur
// l'objet, sélecteur SVG avec aperçu, plan de coupe et points de mesure dans la
// vue 3D (hauteur de coupe depuis le plateau), recherche « + Réglage » par
// pertinence, Espace pour renommer, aide raccourcis en français, onglet
// « Aperçu » et nom du projet à jour dans l'en-tête.
import { expect, test, type Page } from '@playwright/test';
import { bootstrap, importModel } from '../lib/app';

test.use({ locale: 'fr-FR' });

const SVG =
	'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M0 0H10V10H0Z"/></svg>';

async function selectFirst(page: Page) {
	await page.getByTestId('object-name').first().click();
}

async function clickObject(page: Page, dx = 0, dy = 0) {
	const canvas = page.getByTestId('scene-canvas');
	await expect(canvas).toHaveAttribute('data-object-anchor', /^\d+,\d+$/);
	const [x, y] = (await canvas.getAttribute('data-object-anchor'))!.split(',').map(Number);
	const box = (await canvas.boundingBox())!;
	await page.mouse.click(box.x + x + dx, box.y + y + dy);
}

test('texte en relief avec un objet sélectionné : pièce posée sur l’objet (gizmo Emboss) @F:gizmo.emboss', async ({
	page
}) => {
	test.setTimeout(240_000);
	await bootstrap(page);
	await importModel(page, 'cube20');
	await selectFirst(page);
	await page.getByTestId('tool-emboss').click();
	await expect(page.getByTestId('emboss-on-object')).toBeChecked();
	await page.getByTestId('emboss-text').fill('Web');
	await page.getByTestId('emboss-submit').click();
	await expect(page.getByTestId('editor-message')).toContainText('ajoutée sur', {
		timeout: 120_000
	});
	// Toujours un seul objet ; le texte est une pièce de cet objet.
	await expect(page.getByTestId('object-row')).toHaveCount(1);
	await expect(page.getByTestId('object-volume').filter({ hasText: 'Texte « Web »' })).toHaveCount(
		1
	);
});

test('forme SVG : bouton de choix de fichier, nom et aperçu de la forme @F:gizmo.svg', async ({ page }) => {
	await bootstrap(page);
	await page.getByTestId('tool-svg').click();
	await expect(page.getByTestId('emboss-svg-pick')).toHaveText('Choisir un fichier SVG…');
	await expect(page.getByTestId('emboss-svg-name')).toHaveText('Aucun fichier');
	await page.getByTestId('emboss-svg-file').setInputFiles({
		name: 'carre.svg',
		mimeType: 'image/svg+xml',
		buffer: Buffer.from(SVG)
	});
	await expect(page.getByTestId('emboss-svg-name')).toHaveText('carre.svg');
	await expect(page.getByTestId('emboss-svg-preview')).toBeVisible();
	await expect(page.getByTestId('emboss-submit')).toBeEnabled();
});

test('coupe : plan dessiné dans la vue, position en hauteur depuis le plateau, pas de faux champ connecteurs @F:gizmo.cut', async ({
	page
}) => {
	await bootstrap(page);
	await importModel(page, 'cube20');
	await selectFirst(page);
	await page.getByTestId('tool-cut').click();
	const scene = page.locator('[data-guide-plane]');
	// Milieu du cube de 20 mm posé sur le plateau : hauteur 10, bornes 0–20.
	await expect(page.getByTestId('cut-offset-input')).toHaveValue('10');
	await expect(page.getByTestId('cut-range')).toContainText('0 – 20');
	await expect(scene).toHaveAttribute('data-guide-plane', /,10\.00$/);
	await page.getByTestId('cut-offset-input').fill('12.5');
	await expect(scene).toHaveAttribute('data-guide-plane', /,12\.50$/);
	await expect(page.getByTestId('cut-connectors-note')).toBeVisible();
	await expect(page.getByLabel('Pas des connecteurs (mm, 0 = aucun)')).toHaveCount(0);
	const [res] = await Promise.all([
		page.waitForResponse((r) => r.request().method() === 'POST' && /\/cut$/.test(r.url())),
		page.getByTestId('cut-apply').click()
	]);
	const body = res.request().postDataJSON() as { plane: { point: number[] } };
	// Le serveur reçoit toujours le décalage dans le repère recentré du maillage.
	expect(body.plane.point[2]).toBeCloseTo(2.5, 6);
	// Outil fermé : plus de plan dessiné.
	await expect(page.locator('[data-guide-plane]')).toHaveCount(0);
});

test('mesure : les deux points cliqués sont dessinés dans la vue @F:gizmo.measure', async ({ page }) => {
	await bootstrap(page);
	await importModel(page, 'cube20');
	await selectFirst(page);
	await page.getByTestId('tool-measure').click();
	await clickObject(page, -8);
	await expect(page.locator('[data-guide-points]')).toHaveAttribute('data-guide-points', '1');
	await clickObject(page, 8);
	await expect(page.getByTestId('measure-distance')).toBeVisible();
	await expect(page.locator('[data-guide-points]')).toHaveAttribute('data-guide-points', '2');
});

test('« + Réglage » : résultats par pertinence et catégories traduites @F:editor.object-panel', async ({ page }) => {
	await bootstrap(page);
	await importModel(page, 'cube20');
	await selectFirst(page);
	await page.getByTestId('object-settings-add').first().click();
	const results = page.getByTestId('object-settings-results');
	// En-têtes de catégorie traduits (msgstr Orca), y compris en mode Expert.
	await page.getByTestId('object-settings-search').fill('wall');
	const headers = results.locator('li[role=presentation]');
	await expect(headers.first()).toBeVisible();
	for (const h of await headers.allTextContents())
		expect(h).not.toMatch(/Layers and Perimeters|Extruders|Machine limits|^Other/);
	await page.getByTestId('object-settings-search').fill('densité');
	// Premier résultat : un réglage dont le libellé commence par la requête.
	await expect(page.getByTestId('object-settings-result').first()).toContainText(/^Densité/);
});

test('liste d’objets : Espace (ou F2) renomme l’objet sélectionné @F:shortcut.space', async ({ page }) => {
	await bootstrap(page);
	await importModel(page, 'cube20');
	const name = page.getByTestId('object-name').first();
	await name.click();
	await name.press('Space');
	const input = page.getByTestId('object-rename-input');
	await expect(input).toBeFocused();
	await input.fill('Cube témoin');
	await input.press('Enter');
	await expect(page.getByTestId('object-name').first()).toContainText('Cube témoin');
	await page.getByTestId('object-name').first().press('F2');
	await expect(page.getByTestId('object-rename-input')).toBeVisible();
});

test('aide « Raccourcis clavier » en français (textes Orca) @F:shortcut.x @F:menu.keyboard-shortcuts', async ({ page }) => {
	await bootstrap(page);
	await page.getByTestId('scene-canvas').click({ position: { x: 40, y: 40 } });
	await page.keyboard.press('Shift+?');
	const dialog = page.getByRole('dialog', { name: 'Raccourcis clavier' });
	await expect(dialog).toBeVisible();
	await expect(dialog).toContainText('Nouveau projet');
	await expect(dialog).toContainText('Raccourcis globaux');
	await expect(dialog).not.toContainText('Arrange all objects');
});

test.describe('grand écran (1920 px)', () => {
	test.use({ viewport: { width: 1920, height: 1080 } });
	test('onglet « Aperçu » et nom du projet à jour dans l’en-tête', async ({ page }) => {
		await bootstrap(page);
		await importModel(page, 'cube20');
		const nav = page.getByRole('navigation', { name: "Vues de l'éditeur" });
		await expect(nav.getByRole('button', { name: 'Aperçu' })).toBeVisible();
		await expect(nav).not.toContainText('Prévisualisation');
		// Projet « Sans titre » nommé d'après le premier fichier importé (Orca).
		await expect(page.getByTestId('project-name')).not.toHaveText(/Sans titre|Untitled/);
	});
});
