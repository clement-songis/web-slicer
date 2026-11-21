// Menu Calibration › Retraction test (Plater::calib_retraction d'OrcaSlicer) :
// projet « tour de rétraction » avec le mode calibration du moteur
// (`scene.calibration` → SliceRequest.calib → Print::set_calib_params). Le
// G-code doit porter, à chaque couche, la longueur de rétraction écrite par
// GCode.cpp (« ; Calib_Retraction_tower: … length: ») croissant par paliers
// d'un pas par millimètre au-dessus de 0,4 mm.
import { expect, test } from '@playwright/test';
import { bootstrap, downloadGcode, sliceInUi } from '../lib/app';

test('Calibration › Rétraction : longueur croissante par paliers @F:editor.calibration-retraction', async ({ page }) => {
	test.setTimeout(420_000);
	const sourceId = await bootstrap(page);

	await page.getByRole('button', { name: 'Calibration', exact: true }).click();
	await page.getByRole('menuitem', { name: 'Test de rétraction' }).click();
	const dialog = page.getByTestId('engine-calib-dialog');
	await expect(dialog).toBeVisible();
	await dialog.getByTestId('calib-start').fill('0.5');
	await dialog.getByTestId('calib-end').fill('1.5');
	await dialog.getByTestId('calib-step').fill('0.1');
	await dialog.getByTestId('engine-calib-confirm').click();

	await expect(page).not.toHaveURL(new RegExp(sourceId), { timeout: 180_000 });
	await expect(page.getByTestId('object-row')).toHaveCount(1, { timeout: 180_000 });
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 120_000 });

	await sliceInUi(page);
	const gcode = await downloadGcode(page);

	const lengths = [...gcode.matchAll(/^; Calib_Retraction_tower: Z_HEIGHT: [\d.]+, length:([\d.]+)/gm)].map(
		(m) => Number(m[1])
	);
	expect(lengths.length, 'une ligne de calibration par couche').toBeGreaterThan(20);
	expect(lengths[0]).toBeCloseTo(0.5, 6);
	for (let i = 1; i < lengths.length; i++) expect(lengths[i]).toBeGreaterThanOrEqual(lengths[i - 1]);
	const distinct = [...new Set(lengths.map((l) => l.toFixed(3)))].map(Number);
	// Tour coupée à 1,4 + (1,5 − 0,5)/0,1 mm : paliers 0,5 → 1,5.
	expect(distinct.length).toBeGreaterThanOrEqual(10);
	expect(distinct.at(-1)).toBeCloseTo(1.5, 1);
	for (let i = 1; i < distinct.length; i++) expect(distinct[i] - distinct[i - 1]).toBeCloseTo(0.1, 6);
	// Les rétractions réelles (G1 E-…) suivent la longueur de palier.
	expect(gcode).toMatch(/^G1 E-1\.[0-5]/m);
});
