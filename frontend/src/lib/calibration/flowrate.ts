// Calibration du débit (menu Calibration › Flow rate, passes 1 et 2) : miroir
// de Plater::calib_flowrate + adjust_settings_for_flowrate_calib (Orca 2.4.1).
// Le 3MF d'Orca (resources/calib/filament_flow/flowrate-test-passN.3mf) fournit
// 9 (passe 1) ou 10 (passe 2) objets nommés `flowrate_<x>` ; chaque objet reçoit
// `print_flow_ratio = 1 + x/100` et les réglages de surface d'Orca. La tour est
// mise à l'échelle en Z pour faire 10 couches. Aucun mode moteur requis : les
// réglages par objet suffisent (`calib_flowrate_topinfill_special_order` inclus).
import { flowMm3PerMm } from './engine-calib';

export type FlowPass = 1 | 2;
export type FlowPattern = 'archimedeanchords' | 'monotonic';
export interface FlowOptions {
	/** YOLO : modificateurs absolus autour du ratio filament courant. */
	linear?: boolean;
	pattern?: FlowPattern;
}

export const FLOW_MODELS: Record<FlowPass, string> = {
	1: 'calib/filament_flow/flowrate-test-pass1.3mf',
	2: 'calib/filament_flow/flowrate-test-pass2.3mf'
};
export const LINEAR_FLOW_MODELS: Record<FlowPass, string> = {
	1: 'calib/filament_flow/Orca-LinearFlow.3mf',
	2: 'calib/filament_flow/Orca-LinearFlow_fine.3mf'
};

/** `flowrate_m5` → -5, `flowrate_10` → 10 (préfixe de 9 caractères). */
export function flowModifierFromName(name: string): number | null {
	const m = /flowrate_(m?)(\d+(?:\.\d+)?)$/.exec(name.replace(/\.3mf$/i, '').trim());
	if (!m) return null;
	return (m[1] ? -1 : 1) * Number(m[2]);
}

export interface FlowContext {
	nozzleDiameter: number;
	/** `initial_layer_print_height` du process. */
	initialLayerHeight: number;
	internalSolidInfillSpeed: number;
	topSurfaceSpeed: number;
	filamentMaxVolumetricSpeed: number;
	/** `filament_flow_ratio`, base des variantes YOLO. */
	filamentFlowRatio: number;
}

export interface FlowPlan {
	pass: FlowPass;
	linear: boolean;
	baseFlowRatio: number;
	projectName: string;
	model: string;
	layerHeight: number;
	firstLayerHeight: number;
	/** Facteur d'échelle (x, y, z) appliqué à chaque objet. */
	scale: [number, number, number];
	process: Record<string, unknown>;
	printer: Record<string, unknown>;
	/** Réglages objet communs (hors `print_flow_ratio`). */
	objectSettings: Record<string, unknown>;
}

export function planFlowRate(
	pass: FlowPass,
	ctx: FlowContext,
	options: FlowOptions = {}
): FlowPlan {
	const linear = options.linear ?? false;
	const baseFlowRatio = ctx.filamentFlowRatio > 0 ? ctx.filamentFlowRatio : 1;
	const d = ctx.nozzleDiameter > 0 ? ctx.nozzleDiameter : 0.4;
	const lh = d / 2;
	const first = Math.max(ctx.initialLayerHeight || 0, lh);
	const zscale = (first + 9 * lh) / 2;
	const xy = d / 0.6;
	const maxFactor = linear
		? (baseFlowRatio + (pass === 2 ? 0.035 : 0.05)) / baseFlowRatio
		: pass === 1
			? 1.2
			: 1;
	const maxInfill = ctx.filamentMaxVolumetricSpeed / (flowMm3PerMm(d * 1.2, lh) * maxFactor);
	const speed = (v: number) => Math.floor(Math.min(v > 0 ? v : maxInfill, maxInfill));
	return {
		pass,
		linear,
		baseFlowRatio,
		projectName: linear
			? `Orca YOLO Flow Calibration${pass === 2 ? ' - Perfectionist version' : ''}`
			: `Flowrate Test - Pass${pass}`,
		model: (linear ? LINEAR_FLOW_MODELS : FLOW_MODELS)[pass],
		layerHeight: lh,
		firstLayerHeight: first,
		scale: xy > 1.2 ? [xy, xy, zscale] : [1, 1, zscale],
		process: {
			layer_height: lh,
			alternate_extra_wall: false,
			initial_layer_print_height: first,
			reduce_crossing_wall: true,
			enable_wrapping_detection: false,
			max_volumetric_extrusion_rate_slope: 0
		},
		printer: { resonance_avoidance: false },
		objectSettings: {
			wall_loops: 1,
			only_one_wall_top: true,
			thick_internal_bridges: false,
			enable_extra_bridge_layer: 'disabled',
			internal_bridge_density: 100,
			sparse_infill_density: 35,
			min_width_top_surface: '100%',
			bottom_shell_layers: 2,
			top_shell_layers: 5,
			top_shell_thickness: 0,
			bottom_shell_thickness: 0,
			detect_thin_wall: true,
			filter_out_gap_fill: 0,
			sparse_infill_pattern: 'rectilinear',
			top_surface_line_width: d * 1.2,
			internal_solid_infill_line_width: d * 1.2,
			top_surface_pattern: options.pattern ?? 'monotonic',
			top_solid_infill_flow_ratio: 1,
			infill_direction: 45,
			solid_infill_direction: 135,
			align_infill_direction_to_model: true,
			ironing_type: 'no ironing',
			internal_solid_infill_speed: speed(ctx.internalSolidInfillSpeed),
			top_surface_speed: speed(ctx.topSurfaceSpeed),
			seam_slope_type: 'none',
			gap_fill_target: 'nowhere',
			calib_flowrate_topinfill_special_order: true
		}
	};
}

/** Réglages d'un objet `flowrate_<x>` (null si le nom n'est pas reconnu). */
export function flowObjectSettings(plan: FlowPlan, name: string): Record<string, unknown> | null {
	const modifier = flowModifierFromName(name);
	if (modifier === null) return null;
	return {
		...plan.objectSettings,
		print_flow_ratio: plan.linear
			? (plan.baseFlowRatio + modifier) / plan.baseFlowRatio
			: 1 + modifier / 100
	};
}

interface SceneObjectLike {
	modelId?: string;
	transform?: { position?: number[]; rotation?: number[]; scale?: number[] };
	settings?: Record<string, unknown>;
	[key: string]: unknown;
}

/**
 * Applique le plan à la scène importée : réglages par objet selon le nom du
 * modèle, échelle, et pose sur le plateau (les objets du 3MF y reposent : le
 * centre Z est multiplié par l'échelle Z, comme `ensure_on_bed`).
 */
export function applyFlowPlanToScene(
	scene: { objects?: SceneObjectLike[]; [key: string]: unknown },
	plan: FlowPlan,
	nameOfModel: (modelId: string) => string | undefined
): { scene: typeof scene; tuned: number } {
	let tuned = 0;
	const objects = (scene.objects ?? []).map((object) => {
		const name = object.modelId ? nameOfModel(object.modelId) : undefined;
		const settings = name ? flowObjectSettings(plan, name) : null;
		if (!settings) return object;
		tuned++;
		const t = object.transform ?? {};
		const position = [...(t.position ?? [0, 0, 0])];
		const scale = (t.scale ?? [1, 1, 1]).map((s, i) => s * plan.scale[i]);
		position[2] = (position[2] ?? 0) * plan.scale[2];
		return {
			...object,
			name,
			transform: { ...t, position, scale },
			settings: { ...(object.settings ?? {}), ...settings }
		};
	});
	return { scene: { ...scene, objects }, tuned };
}
