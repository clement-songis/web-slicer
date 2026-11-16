// F7 : un 3MF à trois objets produit par OrcaSlicer (cube.stl, pyramide.stl,
// pont.stl) importé dans un projet donne trois objets nommés, comme Orca.
import { expect, test } from '@playwright/test';
import { bootstrap } from '../lib/app';

test('import 3MF multi-objets : trois objets nommés @F:ui.import-3mf-multi', async ({ page }) => {
	await bootstrap(page);
	await page
		.getByTestId('import-input')
		.setInputFiles('/var/lib/openclaw/workspace/web-slicer/backend/tests/fixtures/trois-orca.3mf');
	await expect(page.getByTestId('object-row')).toHaveCount(3, { timeout: 60_000 });
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
	const names = await page.getByTestId('object-name').allTextContents();
	expect(names.map((n) => n.replace(/^[•\s]+/, '').trim()).sort()).toEqual(['cube.stl', 'pont.stl', 'pyramide.stl']);
	// Persistance : rechargement → toujours trois objets nommés.
	await page.waitForTimeout(2500);
	await page.reload();
	await expect(page.getByTestId('object-row')).toHaveCount(3, { timeout: 60_000 });
});
