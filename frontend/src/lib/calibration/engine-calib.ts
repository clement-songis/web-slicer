// Calibrations « mode moteur » du menu (Retraction_Test_Dlg, VFA_Test_Dlg,
// MaxVolumetricSpeed_Test_Dlg, PA_Calibration_Dlg (motif),
// Input_Shaping_Freq/Damp_Test_Dlg, Cornering_Test_Dlg d'OrcaSlicer 2.4.1) :
// paramètres des dialogues et validation. Le projet est construit par le
// serveur (POST /api/projects/{id}/calibration, cf. server.ts).
import { tr } from '$lib/i18n';

export type EngineCalibKind =
	| 'retraction'
	| 'vfa'
	| 'max_volumetric_speed'
	| 'pa_pattern'
	| 'input_shaping_freq'
	| 'input_shaping_damp'
	| 'cornering';

export interface RangeParams {
	start: number;
	end: number;
	step: number;
	/** Valeur fixe des tests d'input shaping : amortissement (test de
	 *  fréquence) ou fréquence X en Hz (test d'amortissement). */
	fixed?: number;
	/** Test d'amortissement : fréquence Y (Hz), sinon = X. */
	fixedY?: number;
	/** Test de fréquence : plage Y (Hz), sinon = plage X. */
	startY?: number;
	endY?: number;
	/** Motif PA : numéros imprimés, vitesses et accélérations testées. */
	printNumbers?: boolean;
	speeds?: number[];
	accelerations?: number[];
	/** Input shaping / virages : 0 ringing_tower, 1 fast_tower_test, 2 SCV-V2. */
	testModel?: number;
	/** `input_shaping_type` d'Orca. */
	shaperType?: string;
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
/** Calibrations avec choix du modèle de tour. */
export const TEST_MODEL_KINDS: readonly EngineCalibKind[] = [
	'input_shaping_freq',
	'input_shaping_damp',
	'cornering'
];

/** Modèles de tour d'Orca (Input_Shaping_*_Test_Dlg, Cornering_Test_Dlg). */
export function testModels(kind: EngineCalibKind): { id: number; label: string }[] {
	const base = [
		{ id: 0, label: 'Ringing Tower' },
		{ id: 1, label: 'Fast Tower' }
	];
	return kind === 'cornering' ? [...base, { id: 2, label: 'SCV-V2' }] : base;
}

/** `input_shaping_type` d'Orca (PrintConfig.cpp), « Disable » exclu. */
export const SHAPER_TYPES = [
	'Default',
	'MZV',
	'ZV',
	'ZVD',
	'ZVDD',
	'ZVDDD',
	'EI',
	'EI2',
	'2HUMP_EI',
	'EI3',
	'3HUMP_EI',
	'DAA'
] as const;

/** « 100, 200 » → [100, 200] ; null si une valeur n'est pas un nombre > 0. */
export function parseNumberList(raw: string): number[] | null {
	const parts = raw
		.split(/[,;\s]+/)
		.map((x) => x.trim())
		.filter(Boolean);
	const values = parts.map(Number);
	return values.every((v) => Number.isFinite(v) && v > 0) ? values : null;
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

/** Libellés = clés anglaises, traduites au rendu. */
export const ENGINE_CALIB_LABELS: Record<EngineCalibKind, string> = {
	retraction: 'Retraction||calibration',
	vfa: 'VFA',
	max_volumetric_speed: 'Max volumetric flow',
	pa_pattern: 'Pressure advance (pattern)',
	input_shaping_freq: 'Input shaping: frequency',
	input_shaping_damp: 'Input shaping: damping',
	cornering: 'Cornering (jerk / junction deviation)'
};

/** Unité affichée de la plage (Début/Fin/Pas). */
export const ENGINE_CALIB_UNITS: Record<EngineCalibKind, string> = {
	retraction: 'mm',
	vfa: 'mm/s',
	max_volumetric_speed: 'mm³/s',
	// Valeur K sans unité (Orca PA_Calibration_Dlg : « Start PA / End PA / PA step »).
	pa_pattern: '',
	input_shaping_freq: 'Hz',
	input_shaping_damp: '',
	cornering: 'mm/s'
};

/** Libellé et unité de la valeur fixe des tests d'input shaping (clés anglaises). */
export const FIXED_VALUE_LABELS: Partial<Record<EngineCalibKind, string>> = {
	input_shaping_freq: 'Damping',
	input_shaping_damp: 'X frequency (Hz)'
};

export function validateRange(p: RangeParams, kind?: EngineCalibKind): string | null {
	if (![p.start, p.end, p.step].every(Number.isFinite)) return tr('numeric values required');
	if (kind && FIXED_VALUE_KINDS.includes(kind) && !Number.isFinite(p.fixed ?? NaN))
		return tr('fixed value required');
	if (kind === 'input_shaping_damp' && (p.start < 0 || p.end > 1))
		return tr('damping must stay between 0 and 1');
	if (kind === 'input_shaping_freq' && ((p.fixed ?? 0) < 0 || (p.fixed ?? 0) > 1))
		return tr('damping must stay between 0 and 1');
	if (p.step <= 0) return tr('the step must be positive');
	if (p.end <= p.start) return tr('the end must be greater than the start');
	if (p.startY !== undefined && p.endY !== undefined && !(p.endY > p.startY))
		return tr('the Y end must be greater than the Y start');
	if (p.fixedY !== undefined && !(Number.isFinite(p.fixedY) && p.fixedY > 0))
		return tr('invalid Y frequency');
	return null;
}
