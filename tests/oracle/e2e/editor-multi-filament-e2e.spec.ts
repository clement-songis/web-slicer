// Multi-filament de bout en bout : 2 objets, 2 filaments (blanc / noir), purge
// calculée par le moteur (libslic3r FlushVolCalculator, comme Orca), tranchage,
// aperçu (tour d'essuyage, légende par filament aux couleurs du projet,
// changements d'outil) et G-code (couleurs du projet).
import { expect, test, type Page } from '@playwright/test';
import { bootstrap, importModel } from '../lib/app';


async function pickColour(page: Page, slot: number, hex: string) {
	await page.getByTestId('filament-colour').nth(slot).evaluate((el, v) => {
		(el as HTMLInputElement).value = v;
		el.dispatchEvent(new Event('input', { bubbles: true }));
		el.dispatchEvent(new Event('change', { bubbles: true }));
	}, hex);
}

test('multi-filament : purge calculée, tranchage, aperçu par filament @F:editor.multi-filament-e2e', async ({ page }) => {
	test.setTimeout(420_000);
	const id = await bootstrap(page);
	await importModel(page, 'cube20');
	await importModel(page, 'cylinder');
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
	await page.getByTestId('add-filament-slot').click();
	await pickColour(page, 0, '#ffffff');
	await pickColour(page, 1, '#000000');
	await page.getByRole('button', { name: 'Objets', exact: true }).click();
	await page.getByLabel('Extrudeur de cylinder.stl').selectOption('2');
	await page.getByRole('button', { name: /^(Traitement|Process)$/ }).click();
	await expect(page.getByTestId('scene-canvas')).toHaveAttribute('data-wipe-tower', /.+/);

	// Purge calculée par le moteur : valeurs affichées = réponse de la route,
	// asymétrie d'Orca (noir → blanc demande plus que blanc → noir).
	await page.getByTestId('open-flush-volumes').click();
	const dlg = page.getByTestId('flush-dialog');
	const compute = dlg.getByTestId('flush-compute');
	await expect(compute).toHaveAttribute('data-available', 'true');
	const [res] = await Promise.all([
		page.waitForResponse((r) => /\/api\/flush-volumes$/.test(r.url())),
		compute.click()
	]);
	expect(res.status()).toBe(200);
	const { matrix } = (await res.json()) as { matrix: number[] };
	expect(matrix).toHaveLength(4);
	await expect(dlg.getByTestId('flush-cell-1-2')).toHaveValue(String(Math.round(matrix[1])));
	await expect(dlg.getByTestId('flush-cell-2-1')).toHaveValue(String(Math.round(matrix[2])));
	expect(matrix[2]).toBeGreaterThan(matrix[1]);
	await dlg.getByTestId('flush-apply').click();

	// Tranchage puis aperçu.
	await page.getByTestId('slice-button').click();
	const preview = page.getByTestId('preview-view');
	await expect(preview).toHaveAttribute('data-layer-count', /^[1-9]\d*$/, { timeout: 300_000 });
	await expect(page.locator('[data-testid=legend-item][data-role="Prime tower"]')).toHaveCount(1);
	await page.getByLabel("Coloration de l'aperçu").selectOption('filament');
	const entries = page.getByTestId('legend-entry');
	await expect(entries).toHaveCount(2);
	await expect(entries.nth(0)).toHaveAttribute('data-color', /255.255.255/);
	await expect(entries.nth(1)).toHaveAttribute('data-color', /\(0.0.0\)/);
	await expect(page.getByTestId('preview-option-toolchange')).toContainText(/[1-9]\d*/);
	// Une seule tête (Voron) : changements de filament, aucun changement d'extrudeur.
	await expect(page.getByTestId('summary-filament-changes')).toHaveText(/[1-9]\d*/);
	await expect(page.getByTestId('summary-extruder-changes')).toHaveCount(0);

	// G-code : couleurs et purge du projet appliquées.
	const project = (await (await page.request.get(`/api/projects/${id}`)).json()) as {
		scene: { projectConfig?: { flush_volumes_matrix?: number[] } };
	};
	expect(project.scene.projectConfig?.flush_volumes_matrix).toEqual(matrix.map((v) => Math.round(v)));
	const jobs = (await (await page.request.get('/api/jobs')).json()) as { project_id: string; gcode_id?: string }[];
	const job = jobs.find((j) => j.project_id === id && j.gcode_id)!;
	const gcode = await (await page.request.get(`/api/gcodes/${job.gcode_id}/download`)).text();
	expect(gcode).toMatch(/^; filament_colour = #ffffff;#000000/im);
});
