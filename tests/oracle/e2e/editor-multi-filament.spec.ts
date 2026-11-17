// Multi-filament (parité Orca, ergonomie propre) : couleur de filament du projet,
// volumes de purge, extrudeur par volume, teinte des objets, accès à la tour
// d'essuyage ; persistance dans la scène (projectConfig, volumes[].extruder).
import { expect, test, type Page } from '@playwright/test';
import { bootstrap, importModel } from '../lib/app';

type Scene = {
	projectConfig?: { filament_colour?: string[]; flush_volumes_matrix?: number[]; flush_multiplier?: number };
	objects: { name?: string; extruder?: number; volumes?: { extruder?: number }[] }[];
};

async function save(page: Page, id: string): Promise<Scene> {
	const [res] = await Promise.all([
		page.waitForResponse((r) => r.request().method() === 'PUT' && /\/api\/projects\/[^/]+$/.test(r.url())),
		page.getByTestId('save-project').click()
	]);
	expect(res.status()).toBeLessThan(300);
	return ((await (await page.request.get(`/api/projects/${id}`)).json()) as { scene: Scene }).scene;
}

async function pickColour(page: Page, slot: number, hex: string) {
	await page.getByTestId('filament-colour').nth(slot).evaluate((el, v) => {
		(el as HTMLInputElement).value = v;
		el.dispatchEvent(new Event('input', { bubbles: true }));
		el.dispatchEvent(new Event('change', { bubbles: true }));
	}, hex);
}

test('multi-filament : couleurs, purge, extrudeur par volume, persistance @F:editor.multi-filament', async ({ page }) => {
	test.setTimeout(240_000);
	const id = await bootstrap(page);
	await importModel(page, 'cube20');
	await importModel(page, 'cylinder');
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
	const scene = page.getByTestId('scene-canvas');

	// Un seul filament : pas d'outils multi-filament, scène neutre.
	await expect(page.getByTestId('multi-filament-tools')).toHaveCount(0);
	await expect(scene).not.toHaveAttribute('data-extruder-colors', /.+/);
	await page.getByTestId('add-filament-slot').click();
	await expect(page.getByTestId('multi-filament-tools')).toBeVisible();
	await expect(scene).toHaveAttribute('data-extruder-colors', '2');

	// Couleur du slot 2 (config projet), retour à la couleur du preset par le menu.
	await pickColour(page, 1, '#e11d48');
	await expect(page.getByTestId('filament-swatch').nth(1)).toHaveAttribute('data-colour', '#e11d48');
	await expect(page.getByTestId('filament-swatch').nth(1)).toHaveAttribute('data-custom', 'true');

	// Volumes de purge : matrice 2×2, multiplicateur.
	await page.getByTestId('open-flush-volumes').click();
	const dlg = page.getByTestId('flush-dialog');
	await expect(dlg.getByTestId('flush-matrix').locator('input')).toHaveCount(2);
	await dlg.getByTestId('flush-cell-1-2').fill('150');
	await dlg.getByTestId('flush-cell-1-2').press('Tab');
	await dlg.getByTestId('flush-multiplier').fill('1.2');
	await expect(dlg.getByTestId('flush-max')).toContainText('336');
	await dlg.getByTestId('flush-apply').click();
	await expect(dlg).toHaveCount(0);

	// Tour d'essuyage : ouvre les réglages d'impression sur l'onglet multimatériau.
	await page.getByTestId('open-prime-tower').click();
	await expect(page.getByTestId('settings-dialog').locator('[data-param=enable_prime_tower]')).toBeVisible();
	await page.keyboard.press('Escape');

	// Objets : extrudeur 2 pour le cylindre, modificateur du cube sur l'extrudeur 2.
	await page.getByRole('button', { name: 'Objets', exact: true }).click();
	await page.getByLabel('Extrudeur de cylinder.stl').selectOption('2');
	await page.getByTestId('object-row').filter({ hasText: 'cube20.stl' }).click({ button: 'right' });
	await page.getByRole('menuitem', { name: /^(Add modifier|Ajouter un modificateur|Modificateur)/i }).first().click();
	await page
		.getByTestId('object-row')
		.filter({ hasText: 'cube20.stl' })
		.getByRole('button', { name: 'cube20.stl', exact: true })
		.click();
	await page.getByTestId('object-volume-extruder').first().selectOption('2');

	let saved = await save(page, id);
	expect(saved.projectConfig).toEqual({
		filament_colour: ['', '#e11d48'],
		flush_volumes_matrix: [0, 150, 280, 0],
		flush_multiplier: 1.2
	});
	expect(saved.objects.find((o) => o.name === 'cylinder.stl')?.extruder).toBe(2);
	expect(saved.objects.find((o) => o.name === 'cube20.stl')?.volumes?.[0]?.extruder).toBe(2);

	// Rechargement : couleur projet restaurée.
	await page.reload();
	await expect(page.getByTestId('filament-swatch').nth(1)).toHaveAttribute('data-colour', '#e11d48', { timeout: 60_000 });

	// Retrait du slot 2 : couleur, ligne/colonne de purge et extrudeurs repassent en auto.
	await page.locator('[data-testid="filament-slot"][data-slot="1"]').getByTestId('remove-filament-slot').click();
	saved = await save(page, id);
	expect(saved.projectConfig?.filament_colour).toBeUndefined();
	expect(saved.projectConfig?.flush_volumes_matrix).toBeUndefined();
	expect(saved.objects.find((o) => o.name === 'cylinder.stl')?.extruder ?? 0).toBe(0);
	expect(saved.objects.find((o) => o.name === 'cube20.stl')?.volumes?.[0]?.extruder).toBeUndefined();
});
