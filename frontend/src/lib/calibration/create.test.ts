import { describe, expect, it } from 'vitest';
import { DEFAULTS, planTemperatureTower } from './calibration';
import {
	createCalibrationProject,
	knownKeys,
	pressureAdvanceTowerCuts,
	recenteredZ,
	temperatureTowerCuts,
	zExtent,
	type CalibrationDeps
} from './create';

describe('découpes de la tour (Plater::calib_temp)', () => {
	it('230 → 190 °C : garde les blocs 54 à 62 (9 blocs de 10 mm)', () => {
		const cuts = temperatureTowerCuts({ start: 230, end: 190 }, 700);
		expect(cuts.keepLowerBelow).toBeCloseTo(630, 2); // (500-190)/5+1 = 63 blocs
		expect(cuts.keepUpperAbove).toBeCloseTo(540, 2); // (500-230)/5 = 54 blocs
		expect(cuts.keepLowerBelow! - cuts.keepUpperAbove!).toBeCloseTo(90, 1);
	});
	it('découpe omise si au-delà du modèle', () => {
		expect(temperatureTowerCuts({ start: 230, end: 190 }, 100)).toEqual({
			keepLowerBelow: null,
			keepUpperAbove: null
		});
	});
	it('Z recentré pour /cut et étendue Z', () => {
		expect(recenteredZ(30, { min: 0, max: 100 })).toBe(-20);
		expect(zExtent(new Float32Array([0, 0, -5, 1, 1, 12, 2, 2, 3]))).toEqual({ min: -5, max: 12 });
	});
	it('knownKeys filtre les clés absentes du registre', () => {
		expect(knownKeys({ layer_height: 0.2, pas_une_cle: 1 })).toEqual({ layer_height: 0.2 });
	});
});

describe('createCalibrationProject', () => {
	it('dérive 3 presets, importe, découpe deux fois et laisse le modèle final orphelin', async () => {
		const calls: string[] = [];
		let saved: Record<string, unknown> = {};
		let n = 0;
		const deps: CalibrationDeps = {
			async createPreset(kind, base, inherits, values) {
				calls.push(`preset:${kind}:${inherits}:${Object.keys(values).sort().join(',')}`);
				return { id: `${kind}-id`, name: base };
			},
			async createProject(name, active) {
				calls.push(
					`project:${name}:${active.printer}:${active.process}:${active.filaments.join()}`
				);
				return { id: 'proj' };
			},
			async fetchModelFile(path) {
				calls.push('fetch:' + path);
				return new File([new Uint8Array(1)], 't.drc');
			},
			async uploadModel() {
				return { id: 'm0' };
			},
			async modelZExtent(id) {
				return id === 'm0' ? { min: 0, max: 700 } : { min: 0, max: 630 };
			},
			async cut(id, z, keep) {
				calls.push(`cut:${id}:${z.toFixed(1)}:${keep}`);
				return { id: `m${++n}` };
			},
			async saveScene(_id, scene) {
				saved = scene as Record<string, unknown>;
			},
			async deletePreset(id) {
				calls.push('delete-preset:' + id);
			},
			async deleteProject(id) {
				calls.push('delete-project:' + id);
			},
			bedCenter: () => [175, 175]
		};
		const ctx = { gcodeFlavor: 'klipper', nozzleDiameter: 0.4, layerChangeGcode: '' };
		const plan = planTemperatureTower(DEFAULTS.temperature, ctx);
		const src = {
			printer: { id: 'p', name: 'Voron' },
			process: { id: 'q', name: '0.20mm' },
			filament: { id: 'f', name: 'PLA' }
		};
		const id = await createCalibrationProject(plan, src, deps, (z) =>
			temperatureTowerCuts(DEFAULTS.temperature, z)
		);
		expect(calls.some((c) => c.startsWith('delete'))).toBe(false);
		expect(id).toBe('proj');
		expect(calls[0]).toMatch(/^preset:machine:Voron:layer_change_gcode/);
		expect(calls[1]).toMatch(
			/^preset:filament:PLA:nozzle_temperature,nozzle_temperature_initial_layer/
		);
		expect(calls[2]).toContain('preset:process:0.20mm:');
		expect(calls[2]).toContain('brim_type');
		expect(calls[3]).toBe('project:Nozzle temperature test:machine-id:process-id:filament-id');
		expect(calls).toContain('cut:m0:280.0:lower'); // 630 − 350
		expect(calls).toContain('cut:m1:225.0:upper'); // 540 − 315
		const objects = saved.objects as { modelId: string; transform: { position: number[] } }[];
		expect(objects).toHaveLength(1);
		expect(objects[0].modelId).toBe('m2');
		expect(objects[0].transform.position).toEqual([175, 175, 315]);
		expect(saved.knownModels).toEqual({ projectId: 'proj', ids: ['m0', 'm1', 'm2'] });

		// Rollback : un échec après création supprime projet puis presets (ordre inverse).
		calls.length = 0;
		deps.cut = async () => {
			throw new Error('découpe impossible');
		};
		await expect(
			createCalibrationProject(plan, src, deps, (z) =>
				temperatureTowerCuts(DEFAULTS.temperature, z)
			)
		).rejects.toThrow('découpe impossible');
		expect(calls.filter((c) => c.startsWith('delete'))).toEqual([
			'delete-project:proj',
			'delete-preset:process-id',
			'delete-preset:filament-id',
			'delete-preset:machine-id'
		]);
	});

	it('tour PA : garde le bas jusqu’à ceil((end−start)/step)+1 mm', () => {
		expect(pressureAdvanceTowerCuts(DEFAULTS.pressure_advance, 200)).toEqual({
			keepLowerBelow: 51,
			keepUpperAbove: null
		});
		expect(pressureAdvanceTowerCuts(DEFAULTS.pressure_advance, 40).keepLowerBelow).toBeNull();
	});
});
