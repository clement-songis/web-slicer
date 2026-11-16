// Calibrations OrcaSlicer (menu « Calibration ») — planification pure.
//
// Orca (Plater::calib_*) crée un projet, charge un modèle de resources/calib,
// applique des surcharges aux presets édités et à l'objet, puis pose un
// `CalibMode` que GCode.cpp exploite à chaque changement de couche
// (GCode::process_layer → « Calibration Layer-specific GCode »).
//
// Notre moteur ne transmet pas (encore) `calib_params` : on reproduit l'effet
// par couche avec le langage de placeholders d'Orca dans `layer_change_gcode`
// (variables `layer_num`, `layer_z`, `total_layer_count`, évaluées par le
// même PlaceholderParser), ajouté APRÈS le G-code de changement de couche
// existant de l'imprimante. Les formules sont celles de GCode.cpp :
//   - Temp tower : interpolate_value_across_layers(start, end, 5)
//   - PA tower   : start + int(print_z) * step
// Les calibrations qui modifient un réglage interne par couche (vitesse de
// paroi VFA/débit max, longueur de rétraction) ne sont pas exprimables en
// G-code portable : elles exigent le passage de `calib_params` au moteur.

export type CalibrationKind = 'temperature' | 'pressure_advance';

export type GcodeFlavor = 'klipper' | 'marlin' | 'marlin2' | 'reprapfirmware' | string;

export interface TemperatureParams {
	/** Température du premier bloc (bas de la tour), °C. Orca : 230 (PLA). */
	start: number;
	/** Température du dernier bloc (haut), °C. Orca : 190 (PLA). */
	end: number;
}

export interface PressureAdvanceParams {
	/** PA de départ (z = 0). Orca : 0. */
	start: number;
	/** PA de fin. Orca : 0.1 (direct drive) / 1.0 (bowden). */
	end: number;
	/** Incrément par millimètre de hauteur. Orca : 0.002 (DD) / 0.02 (bowden). */
	step: number;
}

export interface CalibrationContext {
	/** `gcode_flavor` de l'imprimante (commande PA dépendante du firmware). */
	gcodeFlavor: GcodeFlavor;
	/** Diamètre de buse (mm), pour la hauteur de couche dérivée (Orca : d/2). */
	nozzleDiameter: number;
	/** `layer_change_gcode` actuel de l'imprimante (préservé, complété). */
	layerChangeGcode: string;
	/** Centre du plateau (mm), pour poser la tour. */
	bedCenter?: [number, number];
}

/** Plan d'un projet de calibration prêt à trancher. */
export interface CalibrationPlan {
	kind:
		| CalibrationKind
		| 'retraction'
		| 'vfa'
		| 'max_volumetric_speed'
		| 'pa_pattern'
		| 'input_shaping_freq'
		| 'input_shaping_damp'
		| 'cornering';
	/** Mode calibration moteur (`scene.calibration`), cf. engine-calib.ts. */
	calibration?: Record<string, unknown>;
	/** Largeur de plateau (mm) : échelle X si le modèle dépasse `fitWidth − 10` (débit max). */
	fitWidth?: number;
	/** Nom du projet (libellés d'Orca). */
	projectName: string;
	/** Modèle Orca à importer (chemin relatif à resources/). */
	model: string;
	/** Réglages posés sur l'objet (portée objet). */
	objectSettings: Record<string, unknown>;
	/** Surcharges des presets dérivés (portée globale). */
	process: Record<string, unknown>;
	filament: Record<string, unknown>;
	printer: Record<string, unknown>;
}

/** Valeurs par défaut des dialogues Orca (Temp_Calibration_Dlg, PA_Calibration_Dlg). */
export const DEFAULTS = {
	temperature: { start: 230, end: 190 } satisfies TemperatureParams,
	pressure_advance: { start: 0, end: 0.1, step: 0.002 } satisfies PressureAdvanceParams
};

const MARK = '; web-slicer calibration';

/** Ajoute un bloc de calibration au G-code de couche existant (idempotent). */
export function appendLayerGcode(existing: string, block: string): string {
	const base = existing.split(MARK)[0].replace(/\s+$/, '');
	return (base ? base + '\n' : '') + MARK + '\n' + block;
}

/** Commande PA selon le firmware (GCodeWriter::set_pressure_advance d'Orca). */
export function pressureAdvanceCommand(flavor: GcodeFlavor, valueExpr: string): string {
	if (flavor === 'klipper') return `SET_PRESSURE_ADVANCE ADVANCE={${valueExpr}}`;
	if (flavor === 'reprapfirmware') return `M572 D0 S{${valueExpr}}`;
	return `M900 K{${valueExpr}}`;
}

/**
 * Placeholder Orca reproduisant interpolate_value_across_layers(start, end, 5) :
 * couche ≤ 1 → start ; sinon start + ratio·(end − start), le bord de départ
 * décalé d'un pas puis tronqué au multiple de 5 (trunc = int()).
 */
export function temperatureExpr(start: number, end: number, step = 5): string {
	let s = start;
	let e = end;
	if (s > e) s += step;
	else e += step;
	const ratio = 'layer_num * 1.0 / (total_layer_count - 1)';
	return `int((${s} + ${ratio} * (${e - s})) / ${step}) * ${step}`;
}

/** Évalue la formule de température (miroir JS, pour tests et aperçu UI). */
export function temperatureAt(
	start: number,
	end: number,
	layer: number,
	layerCount: number,
	step = 5
): number {
	if (layer <= 1) return start;
	let s = start;
	let e = end;
	if (s > e) s += step;
	else e += step;
	const v = s + (layer / (layerCount - 1)) * (e - s);
	return Math.trunc(v / step) * step;
}

/** Évalue le PA à une hauteur (miroir JS de GCode.cpp). */
export function pressureAdvanceAt(p: PressureAdvanceParams, printZ: number): number {
	return p.start + Math.trunc(printZ) * p.step;
}

/** Hauteur de tour PA (Orca : ceil((end − start)/step) + 1 mm). */
export function pressureAdvanceTowerHeight(p: PressureAdvanceParams): number {
	return Math.ceil((p.end - p.start) / p.step) + 1;
}

export function validateTemperature(p: TemperatureParams): string | null {
	if (!(p.start >= 170 && p.start <= 350 && p.end >= 170 && p.end <= 350))
		return 'Températures hors plage (170–350 °C).';
	if (p.start <= p.end) return 'La température de départ doit être supérieure à celle de fin.';
	return null;
}

export function validatePressureAdvance(p: PressureAdvanceParams): string | null {
	if (!(p.step > 0)) return 'Le pas doit être positif.';
	if (!(p.end > p.start)) return 'Le PA de fin doit être supérieur au PA de départ.';
	if (pressureAdvanceTowerHeight(p) > 300) return 'Tour trop haute (> 300 mm) : augmentez le pas.';
	return null;
}

/** Tour de température (Plater::calib_temp). */
export function planTemperatureTower(
	p: TemperatureParams,
	ctx: CalibrationContext
): CalibrationPlan {
	const err = validateTemperature(p);
	if (err) throw new Error(err);
	const layer = ctx.nozzleDiameter / 2;
	return {
		kind: 'temperature',
		projectName: 'Nozzle temperature test',
		model: 'calib/temperature_tower/temperature_tower.drc',
		objectSettings: {
			layer_height: layer,
			brim_type: 'outer_only',
			brim_width: 5,
			brim_object_gap: 0,
			alternate_extra_wall: false,
			seam_slope_type: 'none',
			overhang_reverse: false,
			precise_z_height: false
		},
		process: { enable_wrapping_detection: false, initial_layer_print_height: layer },
		filament: { nozzle_temperature: [p.start], nozzle_temperature_initial_layer: [p.start] },
		printer: {
			resonance_avoidance: false,
			layer_change_gcode: appendLayerGcode(
				ctx.layerChangeGcode,
				// Couches 0-1 : température de départ (branche m_layer_index <= 1 d'Orca).
				`{if layer_num <= 1}M104 S${p.start}{else}M104 S{${temperatureExpr(p.start, p.end)}}{endif}`
			)
		}
	};
}

/** Tour de pressure advance (Plater::_calib_pa_tower). */
export function planPressureAdvanceTower(
	p: PressureAdvanceParams,
	ctx: CalibrationContext
): CalibrationPlan {
	const err = validatePressureAdvance(p);
	if (err) throw new Error(err);
	return {
		kind: 'pressure_advance',
		projectName: 'Pressure advance test',
		model: 'calib/pressure_advance/tower_with_seam.drc',
		objectSettings: {
			alternate_extra_wall: false,
			seam_position: 'back',
			wall_loops: 2,
			top_shell_layers: 0,
			bottom_shell_layers: 0,
			sparse_infill_density: 0,
			brim_type: 'brim_ears',
			brim_object_gap: 0,
			brim_ears_max_angle: 135,
			brim_width: 6,
			seam_slope_type: 'none'
		},
		process: { enable_wrapping_detection: false, max_volumetric_extrusion_rate_slope: 0 },
		filament: { slow_down_layer_time: [1] },
		printer: {
			layer_change_gcode: appendLayerGcode(
				ctx.layerChangeGcode,
				pressureAdvanceCommand(ctx.gcodeFlavor, `${p.start} + int(layer_z) * ${p.step}`)
			)
		}
	};
}
