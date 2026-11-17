// Tour d'essuyage sur le plateau (parité Orca : estimation PartPlate, glisser,
// position wipe_tower_x/y du projet par plateau) et bouton « Calculer depuis
// les couleurs » désactivé tant que la route moteur n'existe pas.
import { expect, test, type Page } from '@playwright/test';
import { bootstrap, importModel } from '../lib/app';

const towerX = async (page: Page) =>
	Number(((await page.getByTestId('scene-canvas').getAttribute('data-wipe-tower')) ?? 'NaN').split(',')[0]);

test('tour d essuyage : affichée, déplaçable, persistée @F:editor.wipe-tower', async ({ page }) => {
	test.setTimeout(240_000);
	const id = await bootstrap(page);
	await importModel(page, 'cube20');
	await importModel(page, 'cylinder');
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
	const scene = page.getByTestId('scene-canvas');
	await page.getByTestId('add-filament-slot').click();
	// Un seul filament utilisé sur le plateau : pas de tour (comme Orca).
	await expect(scene).not.toHaveAttribute('data-wipe-tower', /.+/);
	await page.getByRole('button', { name: 'Objets', exact: true }).click();
	await page.getByLabel('Extrudeur de cylinder.stl').selectOption('2');
	await expect(scene).toHaveAttribute('data-wipe-tower', /^[\d.]+,[\d.]+,[1-9][\d.]*,[1-9][\d.]*$/);

	// Position exacte au clavier.
	await page.getByRole('button', { name: /^(Traitement|Process)$/ }).click();
	await page.getByTestId('wipe-tower-x').fill('100');
	await page.getByTestId('wipe-tower-x').press('Enter');
	await expect.poll(() => towerX(page)).toBe(100);

	// Glisser sur le plateau (ancre écran de la tour).
	const handle = page.getByTestId('wipe-tower-handle');
	// Point de saisie stable : la caméra amortie (OrbitControls) et le
	// redimensionnement de la colonne peuvent encore le déplacer juste après la
	// saisie de X ; on attend deux lectures identiques.
	const grab = async () => (await handle.getAttribute('data-grab')) ?? '';
	await expect(handle).toHaveAttribute('data-grab', /^\d+,\d+$/);
	let last = '';
	await expect
		.poll(async () => {
			const now = await grab();
			const stable = now === last;
			last = now;
			return stable;
		}, { intervals: [250] })
		.toBe(true);
	const [cx, cy] = last.split(',').map(Number);
	// Survol confirmé par la tour elle-même avant d'appuyer (raycast threlte).
	await page.mouse.move(cx - 2, cy);
	await page.mouse.move(cx, cy, { steps: 3 });
	await expect(handle).toHaveAttribute('data-hover', 'true');
	await page.mouse.down();
	await expect(handle).toHaveAttribute('data-dragging', 'true');
	await page.mouse.move(cx + 60, cy, { steps: 8 });
	await page.mouse.up();
	await expect.poll(() => towerX(page)).not.toBe(100);
	const moved = await towerX(page);

	// Persistée dans la config projet (plateau 1), restaurée au rechargement.
	const [res] = await Promise.all([
		page.waitForResponse((r) => r.request().method() === 'PUT' && /\/api\/projects\/[^/]+$/.test(r.url())),
		page.getByTestId('save-project').click()
	]);
	expect(res.status()).toBeLessThan(300);
	const project = (await (await page.request.get(`/api/projects/${id}`)).json()) as {
		scene: { projectConfig?: { wipe_tower_x?: number[]; wipe_tower_y?: number[] } };
	};
	expect(project.scene.projectConfig?.wipe_tower_x?.[0]).toBeCloseTo(moved, 1);
	await page.reload();
	await expect.poll(() => towerX(page), { timeout: 60_000 }).toBeCloseTo(moved, 1);

	// Calcul automatique de la purge : désactivé tant que la route moteur manque.
	await page.getByTestId('open-flush-volumes').click();
	const compute = page.getByTestId('flush-dialog').getByTestId('flush-compute');
	await expect(compute).not.toHaveAttribute('data-available', 'unknown');
	if ((await compute.getAttribute('data-available')) === 'false') await expect(compute).toBeDisabled();
	await page.keyboard.press('Escape');
});
