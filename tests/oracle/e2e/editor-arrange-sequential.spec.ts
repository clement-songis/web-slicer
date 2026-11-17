// « Disposer » en objet par objet (parité Orca ArrangeJob, is_seq_print) : le moteur
// écarte les objets du rayon de dégagement de la tête et fixe l'ordre d'impression
// (print_order) ; la liste d'objets, les étiquettes « Séquence #N » et le G-code le
// suivent, et le tranchage objet par objet est accepté.
import { expect, test, type Page } from '@playwright/test';
import { bootstrap, importModel } from '../lib/app';

const names = async (page: Page) =>
	(await page.getByTestId('object-name').allInnerTexts()).map((s) => s.replace('•', '').trim());

test('objet par objet : Disposer applique l ordre du moteur et le tranchage passe @F:editor.arrange-sequential', async ({ page }) => {
	test.setTimeout(300_000);
	const id = await bootstrap(page);
	for (const m of ['cube20', 'cylinder', 'bridge']) await importModel(page, m);
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
	await page.getByRole('button', { name: 'Objets', exact: true }).click();
	await page.getByTestId('plate-settings').getByTestId('plate-setting-print_sequence').selectOption('by object');

	const [req, res] = await Promise.all([
		page.waitForRequest((r) => /\/arrange$/.test(r.url())),
		page.waitForResponse((r) => /\/arrange$/.test(r.url())),
		page.getByRole('button', { name: /^(Disposer|Arranger)/ }).first().click()
	]);
	const body = req.postDataJSON() as { is_seq_print?: boolean; extruder_clearance_radius?: number };
	expect(body.is_seq_print).toBe(true);
	expect(body.extruder_clearance_radius).toBeGreaterThan(0);
	const placements = ((await res.json()) as { placements: { id: string; print_order?: number }[] }).placements;
	expect(placements.map((p) => p.print_order).sort()).toEqual([1, 2, 3]);
	await expect(page.getByTestId('editor-message')).toContainText('ordre d’impression mis à jour');

	// Rangs de la liste = print_order ; une étiquette « #N » par objet sur la scène.
	await expect(page.getByTestId('object-print-order')).toHaveText(['1', '2', '3']);
	await expect(page.getByTestId('object-sequence-label')).toHaveCount(3);
	expect((await page.getByTestId('object-sequence-label').allInnerTexts()).sort()).toEqual(['#1', '#2', '#3']);
	const listed = await names(page);

	// Tranchage objet par objet accepté (plus de « too close to others ») ; G-code
	// dans l'ordre de la liste.
	await page.getByTestId('slice-button').click();
	await expect(page.getByTestId('preview-view')).toHaveAttribute('data-layer-count', /^[1-9]\d*$/, { timeout: 200_000 });
	const jobs = (await (await page.request.get('/api/jobs')).json()) as { project_id: string; gcode_id?: string }[];
	const job = jobs.find((j) => j.project_id === id && j.gcode_id)!;
	const gcode = await (await page.request.get(`/api/gcodes/${job.gcode_id}/download`)).text();
	const seen: string[] = [];
	for (const line of gcode.split('\n')) {
		const m = /^EXCLUDE_OBJECT_START NAME=(\S+?)(?:_id_\d+_copy_\d+)?$/.exec(line);
		if (m && seen[seen.length - 1] !== m[1]) seen.push(m[1]);
	}
	expect(seen.slice(0, 3)).toEqual(listed.map((n) => n.replace(/[^A-Za-z0-9._-]/g, '_')));
});
