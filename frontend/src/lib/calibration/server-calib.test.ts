import { describe, expect, it } from 'vitest';
import {
	engineRequest,
	flowRequest,
	pressureAdvanceRequest,
	SERVER_CALIB_KINDS,
	temperatureRequest
} from './server-calib';

describe('calibrations côté serveur : requêtes', () => {
	it('14 calibrations portées', () => expect(SERVER_CALIB_KINDS).toHaveLength(14));
	it('température, PA tour / lignes', () => {
		expect(temperatureRequest({ start: 230, end: 190 })).toEqual({
			kind: 'temp_tower',
			start: 230,
			end: 190,
			step: 5
		});
		expect(pressureAdvanceRequest({ start: 0, end: 0.1, step: 0.002 }).kind).toBe('pa_tower');
		expect(pressureAdvanceRequest({ start: 0, end: 0.1, step: 0.002 }, 'line').kind).toBe(
			'pa_line'
		);
	});
	it('débit : passes et YOLO, motif', () => {
		expect(flowRequest(1, false, 'monotonic')).toEqual({ kind: 'flow_pass1', option: 'monotonic' });
		expect(flowRequest(2, true, 'archimedeanchords').kind).toBe('flow_yolo_fine');
	});
	it('rétraction, VFA, débit max', () => {
		expect(engineRequest('max_volumetric_speed', { start: 5, end: 20, step: 0.5 })).toEqual({
			kind: 'max_vol_speed',
			start: 5,
			end: 20,
			step: 0.5
		});
		expect(engineRequest('retraction', { start: 0, end: 2, step: 0.1 }).kind).toBe('retraction');
		expect(engineRequest('vfa', { start: 40, end: 200, step: 10 }).kind).toBe('vfa');
	});
	it('motif PA : numéros, vitesses, accélérations', () => {
		expect(
			engineRequest('pa_pattern', {
				start: 0,
				end: 0.08,
				step: 0.005,
				printNumbers: false,
				speeds: [100, 200],
				accelerations: [3000]
			})
		).toEqual({
			kind: 'pa_pattern',
			start: 0,
			end: 0.08,
			step: 0.005,
			print_numbers: false,
			speeds: [100, 200],
			accelerations: [3000]
		});
		const d = engineRequest('pa_pattern', { start: 0, end: 0.08, step: 0.005 });
		expect([d.print_numbers, d.speeds, d.accelerations]).toEqual([true, [], []]);
	});
	it('input shaping : fréquence (plages X/Y, amortissement fixe) et amortissement', () => {
		expect(
			engineRequest('input_shaping_freq', {
				start: 15,
				end: 110,
				step: 1,
				fixed: 0.15,
				startY: 20,
				endY: 90,
				testModel: 1,
				shaperType: 'MZV'
			})
		).toEqual({
			kind: 'is_freq',
			start: 0.15,
			end: 0.15,
			step: 1,
			test_model: 1,
			shaper_type: 'MZV',
			freq_start_x: 15,
			freq_end_x: 110,
			freq_start_y: 20,
			freq_end_y: 90
		});
		const damp = engineRequest('input_shaping_damp', { start: 0, end: 0.4, step: 1, fixed: 30 });
		expect(damp).toMatchObject({
			kind: 'is_damp',
			start: 0,
			end: 0.4,
			shaper_type: 'Default',
			freq_start_x: 30,
			freq_end_x: 30,
			freq_start_y: 30,
			freq_end_y: 30
		});
	});
	it('virages : plage et modèle SCV-V2', () => {
		expect(engineRequest('cornering', { start: 1, end: 15, step: 1, testModel: 2 })).toEqual({
			kind: 'cornering',
			start: 1,
			end: 15,
			step: 1,
			test_model: 2
		});
	});
});
