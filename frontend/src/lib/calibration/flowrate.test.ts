import { describe, expect, it } from 'vitest';
import {
	applyFlowPlanToScene,
	flowModifierFromName,
	flowObjectSettings,
	planFlowRate
} from './flowrate';

const ctx = {
	nozzleDiameter: 0.4,
	initialLayerHeight: 0.2,
	internalSolidInfillSpeed: 250,
	topSurfaceSpeed: 200,
	filamentMaxVolumetricSpeed: 12,
	filamentFlowRatio: 0.98
};

describe('calibration du débit (adjust_settings_for_flowrate_calib)', () => {
	it('modificateur depuis le nom', () => {
		expect(flowModifierFromName('flowrate_m15')).toBe(-15);
		expect(flowModifierFromName('Projet — flowrate_10.3mf')).toBe(10);
		expect(flowModifierFromName('Title')).toBeNull();
	});
	it('10 couches, vitesses bornées par le débit max', () => {
		const plan = planFlowRate(1, ctx);
		expect(plan.scale).toEqual([1, 1, 1]);
		expect(plan.layerHeight).toBe(0.2);
		const max = Math.floor(12 / (0.2 * (0.48 - 0.2 * (1 - Math.PI / 4)) * 1.2));
		expect(plan.objectSettings.internal_solid_infill_speed).toBe(max);
		expect(planFlowRate(2, ctx).objectSettings.top_surface_speed).toBeGreaterThan(max);
	});
	it('réglages par objet + échelle Z + pose', () => {
		const plan = planFlowRate(1, { ...ctx, initialLayerHeight: 0.3 });
		const scene = {
			objects: [
				{ modelId: 'a', transform: { position: [10, 10, 1], scale: [1, 1, 1] } },
				{ modelId: 'b', transform: { position: [0, 0, 1] } }
			]
		};
		const names: Record<string, string> = { a: 'P — flowrate_m5.3mf', b: 'P — Title.3mf' };
		const { scene: out, tuned } = applyFlowPlanToScene(scene, plan, (id) => names[id]);
		expect(tuned).toBe(1);
		const a = out.objects![0] as {
			settings: Record<string, unknown>;
			transform: { scale: number[]; position: number[] };
		};
		expect(a.settings.print_flow_ratio).toBeCloseTo(0.95);
		expect(a.transform.scale[2]).toBeCloseTo((0.3 + 9 * 0.2) / 2);
		expect(a.transform.position[2]).toBeCloseTo((0.3 + 9 * 0.2) / 2);
		expect(out.objects![1]).toBe(scene.objects[1]);
	});
	it('YOLO : modèle, motif et ratios absolus autour du débit filament', () => {
		const plan = planFlowRate(2, ctx, { linear: true, pattern: 'archimedeanchords' });
		expect(plan.model).toBe('calib/filament_flow/Orca-LinearFlow_fine.3mf');
		expect(plan.projectName).toBe('Orca YOLO Flow Calibration - Perfectionist version');
		expect(plan.objectSettings.top_surface_pattern).toBe('archimedeanchords');
		expect(flowObjectSettings(plan, 'flowrate_m0.035')?.print_flow_ratio).toBeCloseTo(
			(0.98 - 0.035) / 0.98
		);
		expect(flowObjectSettings(plan, 'flowrate_0.035')?.print_flow_ratio).toBeCloseTo(
			(0.98 + 0.035) / 0.98
		);
	});
});
