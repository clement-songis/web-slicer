import { describe, expect, it } from 'vitest';
import {
	DEFAULTS,
	appendLayerGcode,
	planPressureAdvanceTower,
	planTemperatureTower,
	pressureAdvanceAt,
	pressureAdvanceCommand,
	pressureAdvanceTowerHeight,
	temperatureAt,
	temperatureExpr,
	validatePressureAdvance,
	validateTemperature
} from './calibration';

const ctx = {
	gcodeFlavor: 'klipper',
	nozzleDiameter: 0.4,
	layerChangeGcode: ';AFTER_LAYER_CHANGE\n;[layer_z]'
};

describe('formules par couche (miroir de GCode.cpp)', () => {
	it('température : couches 0-1 au départ, puis paliers de 5 °C décroissants jusqu’à la fin', () => {
		expect(temperatureAt(230, 190, 0, 100)).toBe(230);
		expect(temperatureAt(230, 190, 1, 100)).toBe(230);
		const seq = Array.from({ length: 100 }, (_, i) => temperatureAt(230, 190, i, 100));
		expect(seq.at(-1)).toBe(190);
		for (const v of seq) expect(v % 5).toBe(0);
		for (let i = 1; i < seq.length; i++) expect(seq[i]).toBeLessThanOrEqual(seq[i - 1]);
		expect(new Set(seq).size).toBe(9); // 230,225,…,190
	});

	it('placeholder de température : division flottante et troncature int()', () => {
		const e = temperatureExpr(230, 190);
		expect(e).toContain('layer_num * 1.0 / (total_layer_count - 1)');
		expect(e).toMatch(/^int\(\(235 \+ .* \* \(-45\)\) \/ 5\) \* 5$/);
	});

	it('PA : start + int(z) × step', () => {
		const p = { start: 0, end: 0.1, step: 0.002 };
		expect(pressureAdvanceAt(p, 0.2)).toBe(0);
		expect(pressureAdvanceAt(p, 10.4)).toBeCloseTo(0.02);
		expect(pressureAdvanceTowerHeight(p)).toBe(51);
	});
});

describe('commande PA par firmware', () => {
	it('Klipper / RRF / Marlin', () => {
		expect(pressureAdvanceCommand('klipper', 'x')).toBe('SET_PRESSURE_ADVANCE ADVANCE={x}');
		expect(pressureAdvanceCommand('reprapfirmware', 'x')).toBe('M572 D0 S{x}');
		expect(pressureAdvanceCommand('marlin2', 'x')).toBe('M900 K{x}');
	});
});

describe('appendLayerGcode', () => {
	it('préserve le G-code existant et reste idempotent', () => {
		const once = appendLayerGcode(';A', 'M104 S{1}');
		expect(once).toBe(';A\n; web-slicer calibration\nM104 S{1}');
		expect(appendLayerGcode(once, 'M104 S{2}')).toBe(';A\n; web-slicer calibration\nM104 S{2}');
		expect(appendLayerGcode('', 'X')).toBe('; web-slicer calibration\nX');
	});
});

describe('plans', () => {
	it('tour de température : surcharges Orca (calib_temp)', () => {
		const plan = planTemperatureTower(DEFAULTS.temperature, ctx);
		expect(plan.model).toBe('calib/temperature_tower/temperature_tower.drc');
		expect(plan.objectSettings).toMatchObject({
			layer_height: 0.2,
			brim_type: 'outer_only',
			brim_width: 5
		});
		expect(plan.filament.nozzle_temperature).toEqual([230]);
		expect(plan.process.initial_layer_print_height).toBe(0.2);
		expect(String(plan.printer.layer_change_gcode)).toMatch(
			/^;AFTER_LAYER_CHANGE\n;\[layer_z\]\n; web-slicer calibration\n\{if layer_num <= 1\}M104 S230\{else\}M104 S\{int\(/
		);
	});

	it('tour de PA : surcharges Orca (_calib_pa_tower) et commande Klipper', () => {
		const plan = planPressureAdvanceTower(DEFAULTS.pressure_advance, ctx);
		expect(plan.model).toBe('calib/pressure_advance/tower_with_seam.drc');
		expect(plan.objectSettings).toMatchObject({
			wall_loops: 2,
			sparse_infill_density: 0,
			brim_type: 'brim_ears'
		});
		expect(String(plan.printer.layer_change_gcode)).toContain(
			'SET_PRESSURE_ADVANCE ADVANCE={0 + int(layer_z) * 0.002}'
		);
	});

	it('validation des paramètres', () => {
		expect(validateTemperature({ start: 190, end: 230 })).toMatch(/supérieure/);
		expect(validateTemperature({ start: 400, end: 190 })).toMatch(/plage/);
		expect(validatePressureAdvance({ start: 0, end: 1, step: 0.001 })).toMatch(/trop haute/);
		expect(() => planTemperatureTower({ start: 190, end: 230 }, ctx)).toThrow();
	});
});
