// Impression objet par objet (parité Orca) : la séquence se choisit par plateau,
// l'ordre de la liste d'objets est l'ordre d'impression (ObjectList::OnDrop :
// glisser un objet sur un autre), affiché en rangs, conservé à la sauvegarde et
// suivi par le G-code (sort_object_instances_by_model_order).
import { expect, test, type Page } from '@playwright/test';
import { bootstrap, importModel } from '../lib/app';

type Project = { scene: { objects: { name?: string }[] } };

async function save(page: Page) {
	const [res] = await Promise.all([
		page.waitForResponse((r) => r.request().method() === 'PUT' && /\/api\/projects\/[^/]+$/.test(r.url())),
		page.getByTestId('save-project').click()
	]);
	expect(res.status()).toBeLessThan(300);
}

const names = async (page: Page) =>
	(await page.getByTestId('object-name').allInnerTexts()).map((s) => s.replace('•', '').trim());

test('objet par objet : choix de la séquence, ordre réglable et persistant @F:editor.print-order', async ({ page }) => {
	test.setTimeout(180_000);
	const id = await bootstrap(page);
	await importModel(page, 'cube20');
	await importModel(page, 'cylinder');
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
	await page.getByRole('button', { name: 'Objets', exact: true }).click();

	// Par couche : pas de rang affiché.
	await expect(page.getByTestId('object-print-order')).toHaveCount(0);
	await page.getByTestId('plate-settings').getByTestId('plate-setting-print_sequence').selectOption('by object');
	await expect(page.getByTestId('object-list-sequential')).toBeVisible();
	await expect(page.getByTestId('object-print-order')).toHaveText(['1', '2']);
	expect(await names(page)).toEqual(['cube20.stl', 'cylinder.stl']);

	// Glisser le 2e sur le 1er : il passe en tête (Orca OnDrop).
	await page
		.getByTestId('object-row')
		.filter({ hasText: 'cylinder.stl' })
		.dragTo(page.getByTestId('object-row').filter({ hasText: 'cube20.stl' }));
	await expect.poll(() => names(page)).toEqual(['cylinder.stl', 'cube20.stl']);

	// Clavier : Alt+↓ redescend l'objet.
	await page.getByRole('button', { name: 'cylinder.stl', exact: true }).focus();
	await page.keyboard.press('Alt+ArrowDown');
	await expect.poll(() => names(page)).toEqual(['cube20.stl', 'cylinder.stl']);
	await page.getByRole('button', { name: 'cube20.stl', exact: true }).focus();
	await page.keyboard.press('Alt+ArrowDown');
	await expect.poll(() => names(page)).toEqual(['cylinder.stl', 'cube20.stl']);

	// Sauvegarde : ordre de la scène (= ordre transmis au moteur) ; réouverture.
	await save(page);
	const project = (await (await page.request.get(`/api/projects/${id}`)).json()) as Project;
	expect(project.scene.objects.map((o) => o.name)).toEqual(['cylinder.stl', 'cube20.stl']);
	await page.reload();
	await page.getByRole('button', { name: 'Objets', exact: true }).click();
	await expect.poll(() => names(page), { timeout: 60_000 }).toEqual(['cylinder.stl', 'cube20.stl']);
	await expect(page.getByTestId('object-print-order')).toHaveText(['1', '2']);
});
