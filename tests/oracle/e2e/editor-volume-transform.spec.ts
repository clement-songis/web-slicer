// Volumes : sélection dans la liste (mise en évidence dans la vue 3D via
// data-selected-volume) et édition par le panneau de transformation existant
// (position dans le repère de l'objet, rotation, échelle × taille), persistées
// dans objects[].volumes[].
import { expect, test } from '@playwright/test';
import { bootstrap, importModel } from '../lib/app';

type Volume = { id: string; offset: number[]; size: number[]; rotation?: number[] };
type Project = { scene: { objects: { volumes?: Volume[] }[] } };

test('volume : panneau de transformation (position, rotation, échelle) @F:editor.volume-transform', async ({ page }) => {
	const id = await bootstrap(page);
	await importModel(page, 'cube20');
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
	await page.getByRole('button', { name: 'Objets', exact: true }).click();
	await page.getByTestId('object-row').first().click({ button: 'right' });
	await page.getByRole('menuitem', { name: /Add modifier|Ajouter un modificateur/i }).click();
	await page.getByTestId('object-volume-select').first().click();
	const panel = page.getByTestId('volume-transform');
	await expect(panel).toBeVisible();
	await expect(page.getByTestId('scene-canvas')).toHaveAttribute('data-selected-volume', /.+/);

	await panel.getByLabel('Position (mm) X').fill('2');
	await panel.getByLabel('Rotation (°) Z').fill('30');
	await panel.getByTestId('transform-uniform-scale').click();
	await panel.getByLabel('Échelle X').fill('200');

	const [res] = await Promise.all([
		page.waitForResponse((r) => r.request().method() === 'PUT' && /\/api\/projects\/[^/]+$/.test(r.url())),
		page.getByTestId('save-project').click()
	]);
	expect(res.status()).toBeLessThan(300);
	const project = (await (await page.request.get(`/api/projects/${id}`)).json()) as Project;
	const volume = project.scene.objects[0].volumes![0];
	expect(volume.offset[0]).toBe(2);
	expect(volume.rotation).toEqual([0, 0, 30]);
	// Pavé par défaut : moitié de l'objet (10 mm) → ×2 en X.
	expect(volume.size).toEqual([20, 10, 10]);
});
