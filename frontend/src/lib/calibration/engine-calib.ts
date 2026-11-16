// Calibrations qui exigent le mode calibration du moteur (`Print::set_calib_params`,
// transmis via `scene.calibration` → SliceRequest.calib) : tour de rétraction,
// VFA, débit volumétrique max. Planification pure, miroir de
// Plater::calib_retraction / calib_VFA / calib_max_vol_speed (OrcaSlicer 2.4.1).
// La réutilisation de `createCalibrationProject` (presets dérivés, import du
// .drc, découpe, scène) est assurée par `toCalibrationPlan`.
import type { CalibrationPlan } from './calibration';
import type { TowerCuts } from './create';

export type EngineCalibKind =
	| 'retraction'
	| 'vfa'
	| 'max_volumetric_speed'
	| 'pa_pattern'
	| 'input_shaping_freq'
	| 'input_shaping_damp'
	| 'cornering';

/** Valeur `mode` attendue par le moteur (CalibMode d'Orca, snake_case). */
export type EngineCalibMode =
	| 'retraction_tower'
	| 'vfa_tower'
	| 'vol_speed_tower'
	| 'pa_pattern'
	| 'input_shaping_freq'
	| 'input_shaping_damp'
	| 'cornering';

export interface RangeParams {
	start: number;
	end: number;
	step: number;
	/** Valeur fixe des tests d'input shaping : amortissement (test de
	 *  fréquence) ou fréquence en Hz (test d'amortissement). */
	fixed?: number;
}

/** `scene.calibration` (lu par le backend, `slicing::calib_from_scene`). */
export interface SceneCalibration extends Omit<RangeParams, 'fixed'> {
	mode: EngineCalibMode;
	print_numbers: boolean;
	freq_start_x?: number;
	freq_end_x?: number;
	freq_start_y?: number;
	freq_end_y?: number;
}

/** Calibrations sans pas (interpolation continue couche par couche). */
export const STEPLESS_KINDS: readonly EngineCalibKind[] = [
	'input_shaping_freq',
	'input_shaping_damp',
	'cornering'
];
/** Calibrations avec une valeur fixe en plus de la plage. */
export const FIXED_VALUE_KINDS: readonly EngineCalibKind[] = [
	'input_shaping_freq',
	'input_shaping_damp'
];

export interface EngineCalibContext {
	nozzleDiameter: number;
	/** `max_layer_height` de l'imprimante (Orca ne fait que le relever). */
	maxLayerHeight: number;
	/** `filament_flow_ratio` (débit max : conversion mm³/s → mm/s). */
	flowRatio?: number;
	/** Largeur du plateau (mm), pour la mise à l'échelle X du débit max. */
	bedWidth?: number;
	/** `gcode_flavor` (jerk Klipper 5 / autres 10 ; junction deviation Marlin 2). */
	flavor?: string;
	/** `machine_max_junction_deviation`[0]. */
	maxJunctionDeviation?: number;
	/** `machine_max_jerk_x`[0] / `machine_max_jerk_y`[0]. */
	maxJerkX?: number;
	maxJerkY?: number;
	/** `enable_pressure_advance`[0] du filament. */
	pressureAdvance?: boolean;
}

export interface EngineCalibPlan {
	kind: EngineCalibKind;
	projectName: string;
	model: string;
	objectSettings: Record<string, unknown>;
	process: Record<string, unknown>;
	filament: Record<string, unknown>;
	printer: Record<string, unknown>;
	calibration: SceneCalibration;
	fitWidth?: number;
	/** Hauteur de découpe (mm depuis le bas, garde le bas), null si inutile. */
	keepLowerBelow: (sizeZ: number) => number | null;
}

/** Valeurs par défaut des dialogues Orca (Retraction_Test_Dlg, VFA_Test_Dlg, MaxVolumetricSpeed_Test_Dlg). */
export const ENGINE_CALIB_DEFAULTS: Record<EngineCalibKind, RangeParams> = {
	retraction: { start: 0, end: 2, step: 0.1 },
	vfa: { start: 40, end: 200, step: 10 },
	max_volumetric_speed: { start: 5, end: 20, step: 0.5 },
	// PA_Calibration_Dlg (motif, extrudeur direct) ; Input_Shaping_*_Test_Dlg ;
	// Cornering_Test_Dlg (jerk / SCV Klipper).
	pa_pattern: { start: 0, end: 0.08, step: 0.005 },
	input_shaping_freq: { start: 15, end: 110, step: 1, fixed: 0.15 },
	input_shaping_damp: { start: 0, end: 0.4, step: 1, fixed: 30 },
	cornering: { start: 1, end: 15, step: 1 }
};

export const ENGINE_CALIB_LABELS: Record<EngineCalibKind, string> = {
	retraction: 'Rétraction',
	vfa: 'VFA',
	max_volumetric_speed: 'Débit volumétrique max',
	pa_pattern: 'Pressure advance (motif)',
	input_shaping_freq: 'Input shaping : fréquence',
	input_shaping_damp: 'Input shaping : amortissement',
	cornering: 'Virages (jerk / junction deviation)'
};

/** Unité affichée de la plage (Début/Fin/Pas). */
export const ENGINE_CALIB_UNITS: Record<EngineCalibKind, string> = {
	retraction: 'mm',
	vfa: 'mm/s',
	max_volumetric_speed: 'mm³/s',
	pa_pattern: 's',
	input_shaping_freq: 'Hz',
	input_shaping_damp: '',
	cornering: 'mm/s'
};

/** Libellé et unité de la valeur fixe des tests d'input shaping. */
export const FIXED_VALUE_LABELS: Partial<Record<EngineCalibKind, string>> = {
	input_shaping_freq: 'Amortissement',
	input_shaping_damp: 'Fréquence (Hz)'
};

/** Junction deviation (Marlin 2) plutôt que jerk : Plater.cpp `has_junction_deviation`. */
export function hasJunctionDeviation(ctx: EngineCalibContext): boolean {
	return ctx.flavor === 'marlin2' && (ctx.maxJunctionDeviation ?? 0) > 0;
}

/** Réglages communs des tours input shaping / virages (Plater::calib_input_shaping_*, Calib_Cornering). */
function shapingTowerPlan(
	ctx: EngineCalibContext,
	jerkOrJd: number | null,
	accel: number
): Pick<EngineCalibPlan, 'objectSettings' | 'process' | 'filament' | 'printer'> {
	const printer: Record<string, unknown> = { resonance_avoidance: false };
	const process: Record<string, unknown> = {
		enable_overhang_speed: false,
		timelapse_type: '0',
		wall_loops: 1,
		top_shell_layers: 0,
		bottom_shell_layers: 1,
		sparse_infill_density: 0,
		detect_thin_wall: false,
		spiral_mode: true,
		spiral_mode_smooth: false,
		bottom_surface_pattern: 'rectilinear',
		outer_wall_speed: 200,
		default_acceleration: accel,
		outer_wall_acceleration: accel,
		precise_z_height: false
	};
	if (hasJunctionDeviation(ctx)) {
		printer.machine_max_junction_deviation = [
			jerkOrJd ?? Math.max(ctx.maxJunctionDeviation ?? 0, 0.25)
		];
		process.default_junction_deviation = 0;
	} else {
		const floor = ctx.flavor === 'klipper' ? 5 : 10;
		printer.machine_max_jerk_x = [jerkOrJd ?? Math.max(ctx.maxJerkX ?? 0, floor)];
		printer.machine_max_jerk_y = [jerkOrJd ?? Math.max(ctx.maxJerkY ?? 0, floor)];
		process.default_jerk = 0;
	}
	const filament: Record<string, unknown> = {
		slow_down_layer_time: [0],
		slow_down_min_speed: [0],
		slow_down_for_layer_cooling: [false]
	};
	if (!ctx.pressureAdvance) {
		filament.enable_pressure_advance = [true];
		filament.pressure_advance = [0];
		filament.adaptive_pressure_advance = [false];
	}
	return {
		objectSettings: { brim_type: 'outer_only', brim_width: 3, brim_object_gap: 0 },
		process,
		filament,
		printer
	};
}

export function validateRange(p: RangeParams, kind?: EngineCalibKind): string | null {
	if (![p.start, p.end, p.step].every(Number.isFinite)) return 'valeurs numériques requises';
	if (kind && FIXED_VALUE_KINDS.includes(kind) && !Number.isFinite(p.fixed ?? NaN))
		return 'valeur fixe requise';
	if (kind === 'input_shaping_damp' && (p.start < 0 || p.end > 1))
		return 'l’amortissement doit rester entre 0 et 1';
	if (kind === 'input_shaping_freq' && ((p.fixed ?? 0) < 0 || (p.fixed ?? 0) > 1))
		return 'l’amortissement doit rester entre 0 et 1';
	if (p.step <= 0) return 'le pas doit être positif';
	if (p.end <= p.start) return 'la fin doit être supérieure au début';
	return null;
}

const EPSILON = 1e-4;
const cutBelow = (h: number) => (sizeZ: number) => (h < sizeZ ? h : null);

/** Hauteur de couche de la tour de rétraction (Plater::calib_retraction). */
export function retractionLayerHeight(nozzle: number): number {
	return nozzle <= 0.1 ? 0.05 : nozzle <= 0.2 ? 0.1 : 0.2;
}

/** Section d'extrusion (mm²) d'un `Flow` rectangle arrondi (libslic3r Flow::mm3_per_mm). */
export function flowMm3PerMm(width: number, height: number): number {
	return height * (width - height * (1 - Math.PI / 4));
}

export function planEngineCalibration(
	kind: EngineCalibKind,
	p: RangeParams,
	ctx: EngineCalibContext
): EngineCalibPlan {
	const err = validateRange(p, kind);
	if (err) throw new Error(err);
	const d = ctx.nozzleDiameter > 0 ? ctx.nozzleDiameter : 0.4;
	const none = () => null;
	if (kind === 'pa_pattern') {
		// Le moteur génère le motif (CalibPressureAdvancePattern) autour d'un
		// cube « poignée » et impose les réglages de Plater::_calib_pa_pattern.
		return {
			kind,
			projectName: 'Pressure Advance Test',
			// Cube versionné (static/calibration) : Orca ajoute une primitive « Cube ».
			model: 'calibration/pa_pattern_handle.stl',
			objectSettings: {},
			process: { overhang_reverse: false, precise_z_height: false },
			filament: {},
			printer: { resonance_avoidance: false },
			calibration: {
				mode: 'pa_pattern',
				start: p.start,
				end: p.end,
				step: p.step,
				print_numbers: true
			},
			keepLowerBelow: none
		};
	}
	if (kind === 'input_shaping_freq' || kind === 'input_shaping_damp') {
		const plan = shapingTowerPlan(ctx, null, 20000);
		const freq = kind === 'input_shaping_freq';
		if (freq) plan.process.layer_height = 0.2;
		plan.printer.input_shaping_emit = false;
		return {
			kind,
			projectName: freq ? 'Input shaping Frequency test' : 'Input shaping Damping test',
			model: 'calib/input_shaping/ringing_tower.drc',
			...plan,
			calibration: freq
				? {
						mode: 'input_shaping_freq',
						start: p.fixed ?? 0,
						end: p.fixed ?? 0,
						step: 1,
						print_numbers: false,
						freq_start_x: p.start,
						freq_end_x: p.end,
						freq_start_y: p.start,
						freq_end_y: p.end
					}
				: {
						mode: 'input_shaping_damp',
						start: p.start,
						end: p.end,
						step: 1,
						print_numbers: false,
						freq_start_x: p.fixed ?? 0,
						freq_end_x: p.fixed ?? 0,
						freq_start_y: p.fixed ?? 0,
						freq_end_y: p.fixed ?? 0
					},
			keepLowerBelow: none
		};
	}
	if (kind === 'cornering') {
		const plan = shapingTowerPlan(ctx, p.end, 2000);
		plan.printer.input_shaping_emit = true;
		plan.printer.input_shaping_type = 'Disable';
		plan.filament.filament_max_volumetric_speed = [200];
		return {
			kind,
			projectName: 'Cornering test',
			model: 'calib/input_shaping/ringing_tower.drc',
			...plan,
			calibration: { mode: 'cornering', start: p.start, end: p.end, step: 1, print_numbers: false },
			keepLowerBelow: none
		};
	}
	if (kind === 'retraction') {
		const lh = retractionLayerHeight(d);
		return {
			kind,
			projectName: 'Retraction',
			model: 'calib/retraction/retraction_tower.drc',
			objectSettings: {
				wall_loops: 2,
				top_shell_layers: 0,
				bottom_shell_layers: 3,
				sparse_infill_density: 0,
				layer_height: lh,
				alternate_extra_wall: false,
				seam_position: 'aligned',
				wall_sequence: 'inner wall/outer wall',
				overhang_reverse: false,
				precise_z_height: false
			},
			process: { enable_wrapping_detection: false, initial_layer_print_height: lh },
			filament: {},
			printer: {
				resonance_avoidance: false,
				use_firmware_retraction: false,
				max_layer_height: [Math.max(ctx.maxLayerHeight, lh)]
			},
			calibration: { mode: 'retraction_tower', ...p, print_numbers: false },
			keepLowerBelow: cutBelow(1.0 + 0.4 + (p.end - p.start) / p.step - EPSILON)
		};
	}
	if (kind === 'vfa') {
		return {
			kind,
			projectName: 'VFA test',
			model: 'calib/vfa/vfa.drc',
			objectSettings: { brim_type: 'outer_only', brim_width: 3, brim_object_gap: 0 },
			process: {
				enable_overhang_speed: false,
				timelapse_type: '0',
				wall_loops: 1,
				alternate_extra_wall: false,
				top_shell_layers: 0,
				bottom_shell_layers: 1,
				sparse_infill_density: 0,
				detect_thin_wall: false,
				spiral_mode: true,
				enable_wrapping_detection: false,
				precise_z_height: false
			},
			filament: { slow_down_layer_time: [0] },
			printer: { resonance_avoidance: false },
			calibration: { mode: 'vfa_tower', ...p, print_numbers: false },
			keepLowerBelow: cutBelow(5 * ((p.end - p.start) / p.step + 1))
		};
	}
	// Débit volumétrique max : la plage (mm³/s) est convertie en vitesse (mm/s)
	// par la section d'extrusion de la tour, comme Orca.
	const lw = d * 1.75;
	const lh = d * 0.8;
	const section = flowMm3PerMm(lw, lh) * (ctx.flowRatio ?? 1);
	return {
		kind,
		projectName: 'Max volumetric speed test',
		model: 'calib/volumetric_speed/SpeedTestStructure.drc',
		objectSettings: {
			enable_overhang_speed: false,
			wall_loops: 1,
			alternate_extra_wall: false,
			top_shell_layers: 0,
			bottom_shell_layers: 0,
			sparse_infill_density: 0,
			outer_wall_line_width: lw,
			layer_height: lh,
			brim_type: 'outer_and_inner',
			brim_width: 5,
			brim_object_gap: 0,
			precise_z_height: false
		},
		process: {
			timelapse_type: '0',
			spiral_mode: true,
			max_volumetric_extrusion_rate_slope: 0,
			enable_wrapping_detection: false
		},
		filament: { filament_max_volumetric_speed: [200], slow_down_layer_time: [0] },
		printer: { resonance_avoidance: false, max_layer_height: [Math.max(ctx.maxLayerHeight, lh)] },
		calibration: {
			mode: 'vol_speed_tower',
			start: p.start / section,
			end: p.end / section,
			step: p.step / section,
			print_numbers: false
		},
		keepLowerBelow: cutBelow((p.end - p.start + 1) / p.step),
		fitWidth: ctx.bedWidth
	};
}

/** Adapte au flux commun `createCalibrationProject` (plan + découpes). */
export function toCalibrationPlan(plan: EngineCalibPlan): {
	plan: CalibrationPlan;
	cuts: TowerCuts;
} {
	return {
		plan: {
			kind: plan.kind,
			projectName: plan.projectName,
			model: plan.model,
			objectSettings: plan.objectSettings,
			process: plan.process,
			filament: plan.filament,
			printer: plan.printer,
			calibration: { ...plan.calibration },
			...(plan.fitWidth ? { fitWidth: plan.fitWidth } : {})
		},
		cuts: (sizeZ) => ({ keepLowerBelow: plan.keepLowerBelow(sizeZ), keepUpperAbove: null })
	};
}

/** Contexte imprimante/filament depuis les valeurs résolues des presets. */
export function engineCalibContext(
	printer: Record<string, unknown>,
	filament: Record<string, unknown> = {}
): EngineCalibContext {
	const first = (v: unknown) => Number(Array.isArray(v) ? v[0] : v);
	const nozzle = first(printer.nozzle_diameter);
	const maxLh = first(printer.max_layer_height);
	const flow = first(filament.filament_flow_ratio);
	const optional = (v: unknown) => {
		const n = first(v);
		return Number.isFinite(n) ? n : undefined;
	};
	const pa = Array.isArray(filament.enable_pressure_advance)
		? filament.enable_pressure_advance[0]
		: filament.enable_pressure_advance;
	const xs = (Array.isArray(printer.printable_area) ? printer.printable_area : [])
		.map((pt) => Number(String(pt).split('x')[0]))
		.filter(Number.isFinite);
	return {
		bedWidth: xs.length ? Math.max(...xs) - Math.min(...xs) : undefined,
		nozzleDiameter: Number.isFinite(nozzle) && nozzle > 0 ? nozzle : 0.4,
		maxLayerHeight: Number.isFinite(maxLh) && maxLh > 0 ? maxLh : 0,
		flowRatio: Number.isFinite(flow) && flow > 0 ? flow : 1,
		flavor: typeof printer.gcode_flavor === 'string' ? printer.gcode_flavor : undefined,
		maxJunctionDeviation: optional(printer.machine_max_junction_deviation),
		maxJerkX: optional(printer.machine_max_jerk_x),
		maxJerkY: optional(printer.machine_max_jerk_y),
		pressureAdvance: pa === true || pa === 1 || pa === '1' || pa === 'true'
	};
}
