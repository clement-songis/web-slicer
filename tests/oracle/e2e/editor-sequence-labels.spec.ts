// Étiquettes « Séquence #N » (parité Orca GLCanvas3D::Labels en objet par objet) :
// au-dessus de chaque objet imprimable, dans l'ordre d'impression, avec le nom
// quand Vue › Afficher les étiquettes est actif, masquables depuis la liste d'objets.
import { expect, test } from '@playwright/test';
import { bootstrap, importModel } from '../lib/app';

test('objet par objet : étiquettes de séquence sur la scène, masquables @F:editor.sequence-labels', async ({ page }) => {
	await bootstrap(page);
	await importModel(page, 'cube20');
	await importModel(page, 'cylinder');
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
	const scene = page.getByTestId('scene-canvas');
	await page.getByRole('button', { name: 'Objets', exact: true }).click();

	// Par couche : aucune étiquette de séquence.
	await expect(page.getByTestId('object-sequence-label')).toHaveCount(0);
	await page.getByTestId('plate-settings').getByTestId('plate-setting-print_sequence').selectOption('by object');
	await expect(scene).toHaveAttribute('data-print-order', '2');
	await expect(page.getByTestId('object-sequence-label')).toHaveText(['#1', '#2']);

	// L'ordre suit la liste (glisser le 2e en tête).
	await page
		.getByTestId('object-row')
		.filter({ hasText: 'cylinder.stl' })
		.dragTo(page.getByTestId('object-row').filter({ hasText: 'cube20.stl' }));
	await expect(page.getByTestId('object-row').first()).toContainText('cylinder.stl');

	// Avec les étiquettes de nom (Vue) : nom + « Séquence n° N ».
	await page.getByRole('button', { name: /^(view|vue|affichage)$/i }).first().click();
	await page.getByRole('menuitem', { name: /show labels|étiquettes/i }).first().click();
	const cyl = page.getByTestId('object-sequence-label').filter({ hasText: 'cylinder.stl' });
	await expect(cyl).toContainText('Séquence n° 1');

	// Masquables : bouton de la liste d'objets.
	const toggle = page.getByTestId('object-list-order-toggle');
	await expect(toggle).toHaveAttribute('aria-pressed', 'true');
	await toggle.click();
	await expect(toggle).toHaveAttribute('aria-pressed', 'false');
	await expect(page.getByTestId('object-sequence-label')).toHaveCount(0);
	await expect(scene).not.toHaveAttribute('data-print-order', /.+/);
	// Les étiquettes de nom restent (comportement Orca par couche).
	await expect(page.getByTestId('object-label')).toHaveCount(2);
});
