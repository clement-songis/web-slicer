import { describe, expect, it } from 'vitest';
import {
	engineCalibContext,
	flowMm3PerMm,
	planEngineCalibration,
	toCalibrationPlan,
	validateRange
} from './engine-calib';

const ctx = { nozzleDiameter: 0.4, maxLayerHeight: 0.28 };

describe('calibrations moteur (Plater::calib_*)', () => {
	it('rétraction : mode moteur, couche 0,2, découpe 1,4 + plage/pas', () => {
		const plan = planEngineCalibration('retraction', { start: 0, end: 2, step: 0.1 }, ctx);
		expect(plan.calibration).toEqual({
			mode: 'retraction_tower',
			start: 0,
			end: 2,
			step: 0.1,
			print_numbers: false
		});
		expect(plan.objectSettings.layer_height).toBe(0.2);
		expect(plan.printer.use_firmware_retraction).toBe(false);
		expect(plan.keepLowerBelow(100)).toBeCloseTo(21.4, 3);
		expect(plan.keepLowerBelow(10)).toBeNull();
	});
	it('VFA : 5 mm par palier, spiral', () => {
		const plan = planEngineCalibration('vfa', { start: 40, end: 200, step: 10 }, ctx);
		expect(plan.calibration.mode).toBe('vfa_tower');
		expect(plan.process.spiral_mode).toBe(true);
		expect(plan.keepLowerBelow(200)).toBe(85);
	});
	it('débit max : conversion mm³/s → mm/s par la section d’extrusion', () => {
		const plan = planEngineCalibration(
			'max_volumetric_speed',
			{ start: 5, end: 20, step: 0.5 },
			ctx
		);
		const section = flowMm3PerMm(0.7, 0.32);
		expect(plan.calibration.start).toBeCloseTo(5 / section);
		expect(plan.calibration.step).toBeCloseTo(0.5 / section);
		expect((plan.printer.max_layer_height as number[])[0]).toBeCloseTo(0.32);
	});
	it('validation', () => {
		expect(validateRange({ start: 1, end: 1, step: 1 })).not.toBeNull();
		expect(validateRange({ start: 0, end: 1, step: 0 })).not.toBeNull();
		expect(() => planEngineCalibration('vfa', { start: 0, end: 1, step: -1 }, ctx)).toThrow();
	});
});

describe('PA motif, input shaping, virages (Plater::calib_*)', () => {
	it('PA motif : mode moteur, poignée cube, numéros imprimés', () => {
		const plan = planEngineCalibration('pa_pattern', { start: 0, end: 0.08, step: 0.005 }, ctx);
		expect(plan.calibration).toEqual({
			mode: 'pa_pattern',
			start: 0,
			end: 0.08,
			step: 0.005,
			print_numbers: true
		});
		expect(plan.model).toBe('calibration/pa_pattern_handle.stl');
		expect(plan.keepLowerBelow(100)).toBeNull();
	});
	it('input shaping fréquence : plage X=Y, amortissement fixe en start, Klipper jerk ≥ 5', () => {
		const plan = planEngineCalibration(
			'input_shaping_freq',
			{ start: 15, end: 110, step: 1, fixed: 0.15 },
			{ ...ctx, flavor: 'klipper', maxJerkX: 2, maxJerkY: 8 }
		);
		expect(plan.calibration).toMatchObject({
			mode: 'input_shaping_freq',
			start: 0.15,
			freq_start_x: 15,
			freq_end_x: 110,
			freq_start_y: 15,
			freq_end_y: 110
		});
		expect(plan.printer.machine_max_jerk_x).toEqual([5]);
		expect(plan.printer.machine_max_jerk_y).toEqual([8]);
		expect(plan.printer.input_shaping_emit).toBe(false);
		expect(plan.process).toMatchObject({ spiral_mode: true, layer_height: 0.2, default_jerk: 0 });
		expect(plan.filament).toMatchObject({ enable_pressure_advance: [true], pressure_advance: [0] });
		expect(plan.model).toBe('calib/input_shaping/ringing_tower.drc');
	});
	it('input shaping amortissement : fréquence fixe, plage 0–1 obligatoire', () => {
		const plan = planEngineCalibration(
			'input_shaping_damp',
			{ start: 0, end: 0.4, step: 1, fixed: 30 },
			{ ...ctx, pressureAdvance: true }
		);
		expect(plan.calibration).toMatchObject({
			start: 0,
			end: 0.4,
			freq_start_x: 30,
			freq_start_y: 30
		});
		expect(plan.filament.enable_pressure_advance).toBeUndefined();
		expect(
			validateRange({ start: 0, end: 1.5, step: 1, fixed: 30 }, 'input_shaping_damp')
		).not.toBeNull();
		expect(validateRange({ start: 15, end: 110, step: 1 }, 'input_shaping_freq')).not.toBeNull();
	});
	it('virages : junction deviation Marlin 2, sinon jerk = fin de plage', () => {
		const marlin = planEngineCalibration(
			'cornering',
			{ start: 0, end: 0.25, step: 1 },
			{ ...ctx, flavor: 'marlin2', maxJunctionDeviation: 0.1 }
		);
		expect(marlin.printer.machine_max_junction_deviation).toEqual([0.25]);
		expect(marlin.process.default_junction_deviation).toBe(0);
		expect(marlin.printer.input_shaping_type).toBe('Disable');
		const klipper = planEngineCalibration(
			'cornering',
			{ start: 1, end: 15, step: 1 },
			{ ...ctx, flavor: 'klipper' }
		);
		expect(klipper.printer.machine_max_jerk_x).toEqual([15]);
		expect(klipper.process.default_acceleration).toBe(2000);
		expect(toCalibrationPlan(klipper).plan.calibration).toMatchObject({
			mode: 'cornering',
			start: 1,
			end: 15
		});
	});
	it('contexte : flavor, jerk, junction deviation et PA lus des presets', () => {
		const c = engineCalibContext(
			{
				gcode_flavor: 'marlin2',
				machine_max_junction_deviation: ['0.08'],
				machine_max_jerk_x: [9]
			},
			{ enable_pressure_advance: ['1'] }
		);
		expect(c).toMatchObject({
			flavor: 'marlin2',
			maxJunctionDeviation: 0.08,
			maxJerkX: 9,
			pressureAdvance: true
		});
	});
});
