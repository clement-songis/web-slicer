// Réglages par objet saisis dans l'UI (panneau ObjectSettingsPanel) : clic
// droit sur un objet → « Add settings » → recherche wall_loops → 5, sur un
// objet sur deux, puis tranchage du projet. Même mesure que
// editor-object-settings.spec.ts : l'objet surchargé a ~2× plus d'extrusions
// de paroi interne (preset wall_loops=3 → 2 parois internes, 5 → 4).
import { expect, test, type Page } from '@playwright/test';
import { bootstrap, importModel } from '../lib/app';

type Project = { id: string; version: number; scene: { objects: { id: string; settings?: Record<string, unknown> }[] } };

async function getProject(page: Page, id: string): Promise<Project> {
	const r = await page.request.get(`/api/projects/${id}`);
	expect(r.status()).toBe(200);
	return r.json();
}

async function sliceGcode(page: Page, id: string): Promise<string> {
	const r = await page.request.post(`/api/projects/${id}/slice`, { data: { plate_index: 0 } });
	expect(r.status(), await r.text()).toBeLessThan(300);
	const job = ((await r.json()) as { jobs: { id: string }[] }).jobs[0];
	let gcodeId: string | null = null;
	await expect
		.poll(
			async () => {
				const j = (await (await page.request.get(`/api/jobs/${job.id}`)).json()) as { status: string; gcode_id?: string };
				if (j.status === 'failed') throw new Error(`tranchage en échec : ${JSON.stringify(j)}`);
				gcodeId = j.gcode_id ?? null;
				return j.status;
			},
			{ timeout: 170_000, intervals: [1000] }
		)
		.toMatch(/^(done|completed|succeeded)$/);
	const g = await page.request.get(`/api/gcodes/${gcodeId}/download`);
	expect(g.status()).toBe(200);
	return g.text();
}

function innerWallMovesByObject(gcode: string): Map<string, number> {
	const counts = new Map<string, number>();
	let object: string | null = null;
	let type = '';
	for (const line of gcode.split('\n')) {
		const start = /^; printing object .* id:(\d+) copy/.exec(line);
		if (start) { object = start[1]; continue; }
		if (line.startsWith('; stop printing object')) { object = null; continue; }
		if (line.startsWith(';TYPE:')) { type = line.slice(6); continue; }
		if (object !== null && type === 'Inner wall' && /^G1 .*E\d|^G1 .*E\.\d/.test(line))
			counts.set(object, (counts.get(object) ?? 0) + 1);
	}
	return counts;
}

test('panneau objet : wall_loops=5 ajouté par l\'UI atteint le G-code @F:editor.object-panel', async ({ page }) => {
	const id = await bootstrap(page);
	for (const m of ['cube20', 'cube20']) {
		await importModel(page, m);
		await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
	}
	await page.getByRole('button', { name: 'Objets', exact: true }).click();
	const rows = page.getByTestId('object-row');
	await expect(rows).toHaveCount(2);

	// Clic droit → « Add settings » : ouvre le panneau en mode ajout.
	await rows.nth(1).click({ button: 'right' });
	await page.getByRole('menuitem', { name: /Add settings|Ajouter des réglages/i }).click();
	const panel = page.getByTestId('object-settings-panel');
	await expect(panel).toBeVisible();
	await panel.getByTestId('object-settings-search').fill('wall_loops');
	await panel.locator('[data-testid="object-settings-result"][data-key="wall_loops"]').click();
	const line = panel.locator('[data-testid="object-setting"][data-key="wall_loops"]');
	await expect(line).toBeVisible();
	const input = line.locator('input').first();
	await input.fill('5');
	await input.press('Tab');

	const [res] = await Promise.all([
		page.waitForResponse((r) => r.request().method() === 'PUT' && /\/api\/projects\/[^/]+$/.test(r.url())),
		page.getByTestId('save-project').click()
	]);
	expect(res.status()).toBeLessThan(300);
	const project = await getProject(page, id);
	const overridden = project.scene.objects.filter((o) => o.settings && 'wall_loops' in o.settings);
	expect(overridden, 'une seule surcharge wall_loops persistée').toHaveLength(1);
	expect(String(overridden[0].settings!.wall_loops)).toBe('5');

	const counts = [...innerWallMovesByObject(await sliceGcode(page, id)).values()].sort((a, b) => a - b);
	expect(counts, 'deux objets attendus dans le G-code').toHaveLength(2);
	expect(counts[1] / counts[0]).toBeGreaterThan(1.6);
	expect(counts[1] / counts[0]).toBeLessThan(2.6);

	// Suppression de la surcharge par l'UI.
	await rows.nth(1).click();
	await page.getByTestId('object-settings-panel').getByTestId('object-setting-remove').click();
	await expect(page.getByTestId('object-settings-panel').getByTestId('object-setting')).toHaveCount(0);
});
