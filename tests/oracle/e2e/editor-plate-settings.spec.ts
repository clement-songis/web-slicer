// Réglages par plateau (parité Orca « Plate settings ») : saisis dans l'UI
// (onglet Objets → Réglages du plateau), persistés dans plates[].settings et
// figés dans le snapshot du tranchage de ce plateau (bloc de config du G-code).
import { expect, test, type Page } from '@playwright/test';
import { bootstrap, importModel } from '../lib/app';

type Plate = { id: string; settings?: Record<string, unknown> };
type Project = { id: string; scene: { plates: { plates: Plate[] } | Plate[] } };

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

const configLine = (gcode: string, key: string) =>
	new RegExp(`^; ${key} = (.*)$`, 'm').exec(gcode)?.[1];

async function save(page: Page) {
	const [res] = await Promise.all([
		page.waitForResponse((r) => r.request().method() === 'PUT' && /\/api\/projects\/[^/]+$/.test(r.url())),
		page.getByTestId('save-project').click()
	]);
	expect(res.status()).toBeLessThan(300);
}

test('réglages du plateau : séquence et vase spirale atteignent le G-code @F:editor.plate-settings', async ({ page }) => {
	const id = await bootstrap(page);
	await importModel(page, 'cube20');
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
	await page.getByRole('button', { name: 'Objets', exact: true }).click();
	const panel = page.getByTestId('plate-settings');
	await expect(panel).toBeVisible();

	// Référence : le plateau suit le profil.
	await save(page);
	const before = await sliceGcode(page, id);
	expect(configLine(before, 'spiral_mode')).toBe('0');

	await panel.getByTestId('plate-setting-print_sequence').selectOption('by object');
	await panel.getByTestId('plate-setting-spiral_mode').selectOption('1');
	await save(page);

	const project = (await (await page.request.get(`/api/projects/${id}`)).json()) as Project;
	const plates = Array.isArray(project.scene.plates) ? project.scene.plates : project.scene.plates.plates;
	expect(plates[0].settings).toEqual({ print_sequence: 'by object', spiral_mode: '1' });

	const after = await sliceGcode(page, id);
	expect(configLine(after, 'print_sequence')).toBe('by object');
	expect(configLine(after, 'spiral_mode')).toBe('1');

	// Retour au profil : la surcharge disparaît du document.
	await panel.getByTestId('plate-setting-spiral_mode').selectOption('');
	await panel.getByTestId('plate-setting-print_sequence').selectOption('');
	await save(page);
	const reset = (await (await page.request.get(`/api/projects/${id}`)).json()) as Project;
	const resetPlates = Array.isArray(reset.scene.plates) ? reset.scene.plates : reset.scene.plates.plates;
	expect(resetPlates[0].settings).toBeUndefined();
});
