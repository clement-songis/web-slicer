// Calibrations Orca (suite) : VFA et débit volumétrique max tranchés en mode
// calibration moteur (vitesse de paroi externe croissante par paliers, posée
// par GCode.cpp à chaque couche), et débit passe 1 (3MF d'Orca, 9 objets avec
// `print_flow_ratio` par objet, 10 couches).
import { expect, test, type Page } from '@playwright/test';
import { bootstrap, downloadGcode, sliceInUi } from '../lib/app';

async function openCalibration(page: Page, item: string) {
	await page.getByRole('button', { name: 'Calibration', exact: true }).click();
	await page.getByRole('menuitem', { name: item, exact: true }).click();
}

async function waitNewProject(page: Page, sourceId: string, objects: number) {
	await expect(page).not.toHaveURL(new RegExp(sourceId), { timeout: 180_000 });
	await expect(page.getByTestId('object-row')).toHaveCount(objects, { timeout: 180_000 });
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 120_000 });
}

/** Vitesse (mm/s) de la paroi externe par couche (Z), d'après le G-code. */
function outerWallSpeeds(gcode: string): { z: number; speed: number }[] {
	const out: { z: number; speed: number }[] = [];
	let z = 0;
	let outer = false;
	let seenLayer = -1;
	// Vitesse modale : Orca l'émet sur une ligne « G1 F… » distincte.
	let feed = 0;
	for (const line of gcode.split('\n')) {
		const fm = /^G1 .*F([\d.]+)/.exec(line);
		if (fm) feed = Number(fm[1]);
		const zm = /^;\s?Z(?:_HEIGHT)?:\s?([\d.]+)/.exec(line);
		if (zm) z = Number(zm[1]);
		const tm = /^;\s?(?:TYPE|FEATURE):\s?(.*)$/.exec(line);
		if (tm) outer = /outer wall/i.test(tm[1]);
		// Dernière vitesse d'extrusion de paroi externe de la couche (en vase,
		// la spirale continue avec la vitesse précédente avant la nouvelle « G1 F »).
		if (outer && feed > 0 && /^G1 .*E[\d.]/.test(line)) {
			if (z !== seenLayer) {
				out.push({ z, speed: feed / 60 });
				seenLayer = z;
			} else out[out.length - 1].speed = feed / 60;
		}
	}
	return out;
}

function expectStepwiseIncrease(points: { z: number; speed: number }[], minSteps: number) {
	const body = points.filter((p) => p.z >= 1);
	expect(body.length).toBeGreaterThan(5);
	for (let i = 1; i < body.length; i++)
		expect(body[i].speed).toBeGreaterThanOrEqual(body[i - 1].speed - 1e-6);
	const distinct = [...new Set(body.map((p) => Math.round(p.speed)))];
	expect(distinct.length, `paliers : ${distinct.join(', ')}`).toBeGreaterThanOrEqual(minSteps);
	return distinct;
}

test('Calibration › VFA : vitesse de paroi externe par paliers de 5 mm @F:editor.calibration-vfa', async ({ page }) => {
	test.setTimeout(420_000);
	const sourceId = await bootstrap(page);
	await openCalibration(page, 'VFA');
	const dialog = page.getByTestId('engine-calib-dialog');
	await dialog.getByTestId('calib-start').fill('40');
	await dialog.getByTestId('calib-end').fill('100');
	await dialog.getByTestId('calib-step').fill('10');
	await dialog.getByTestId('engine-calib-confirm').click();
	await waitNewProject(page, sourceId, 1);
	await sliceInUi(page);
	const gcode = await downloadGcode(page);
	const points = outerWallSpeeds(gcode);
	const distinct = expectStepwiseIncrease(points, 4);
	expect(distinct[0]).toBe(40);
	// Paliers d'Orca : start + floor(print_z / 5) × step. Aux multiples de 5 mm,
	// print_z flottant peut tomber juste sous la frontière (palier précédent).
	for (const p of points.filter((q) => q.z >= 1)) {
		const at = (z: number) => 40 + Math.floor(z / 5) * 10;
		expect([at(p.z), at(p.z - 1e-6)]).toContain(Math.round(p.speed));
	}
});

test('Calibration › débit volumétrique max : vitesse croissante avec la hauteur @F:editor.calibration-max-flow', async ({ page }) => {
	test.setTimeout(420_000);
	const sourceId = await bootstrap(page);
	await openCalibration(page, 'Débit maximal');
	await page.getByTestId('engine-calib-confirm').click();
	await waitNewProject(page, sourceId, 1);
	await sliceInUi(page);
	const points = outerWallSpeeds(await downloadGcode(page));
	const distinct = expectStepwiseIncrease(points, 10);
	// Orca : vitesse = start + print_z × step (mm/s convertis depuis mm³/s par la
	// section × ratio de débit du filament) : linéaire en Z, ~2,5 mm/s par mm.
	const body = points.filter((q) => q.z >= 1);
	const a = body[0];
	const b = body.at(-1)!;
	const slope = (b.speed - a.speed) / (b.z - a.z);
	const section = 0.32 * (0.7 - 0.32 * (1 - Math.PI / 4));
	expect(slope).toBeGreaterThan((0.5 / section) * 0.9);
	expect(slope).toBeLessThan((0.5 / section) * 1.15);
	for (const p of body) expect(Math.abs(p.speed - (a.speed + slope * (p.z - a.z)))).toBeLessThanOrEqual(1.5);
	expect(distinct.at(-1)! - distinct[0]).toBeGreaterThan(20);
});

test('Calibration › débit passe 1 : 9 objets, print_flow_ratio par objet, 10 couches @F:editor.calibration-flow', async ({ page }) => {
	test.setTimeout(420_000);
	const sourceId = await bootstrap(page);
	await openCalibration(page, 'Ratio de débit : passe 1');
	await waitNewProject(page, sourceId, 9);
	const projectId = /projects\/([^/?#]+)/.exec(page.url())![1];
	const project = await (await page.request.get(`/api/projects/${projectId}`)).json();
	const ratios = (project.scene.objects as { settings: { print_flow_ratio?: number } }[])
		.map((o) => o.settings.print_flow_ratio)
		.sort((a, b) => (a ?? 0) - (b ?? 0));
	expect(ratios.map((r) => Number(r?.toFixed(2)))).toEqual([0.8, 0.85, 0.9, 0.95, 1, 1.05, 1.1, 1.15, 1.2]);
	const layers = await sliceInUi(page);
	expect(layers).toBe(10);
	const gcode = await downloadGcode(page);
	expect(gcode).toMatch(/print_flow_ratio/);
});
