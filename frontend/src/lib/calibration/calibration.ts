// Calibrations température / pressure advance (Temp_Calibration_Dlg,
// PA_Calibration_Dlg d'OrcaSlicer) : paramètres et validation des dialogues.
// Le projet est construit par le serveur (POST /api/projects/{id}/calibration,
// portage des Plater::calib_* ; cf. server.ts).
import { tr } from '$lib/i18n';

export type CalibrationKind = 'temperature' | 'pressure_advance';

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
	/** Méthode d'Orca : tour (défaut) ou lignes (pa_line). */
	method?: 'tower' | 'line';
}

/** Valeurs par défaut des dialogues Orca (Temp_Calibration_Dlg, PA_Calibration_Dlg). */
export const DEFAULTS = {
	temperature: { start: 230, end: 190 } satisfies TemperatureParams,
	pressure_advance: { start: 0, end: 0.1, step: 0.002 } satisfies PressureAdvanceParams
};

/** Hauteur de tour PA (Orca : ceil((end − start)/step) + 1 mm). */
export function pressureAdvanceTowerHeight(p: PressureAdvanceParams): number {
	return Math.ceil((p.end - p.start) / p.step) + 1;
}

export function validateTemperature(p: TemperatureParams): string | null {
	if (!(p.start >= 170 && p.start <= 350 && p.end >= 170 && p.end <= 350))
		return tr('Temperatures out of range (170–350 °C).');
	if (p.start <= p.end) return tr('The start temperature must be higher than the end temperature.');
	return null;
}

export function validatePressureAdvance(p: PressureAdvanceParams): string | null {
	if (!(p.step > 0)) return tr('The step must be positive.');
	if (!(p.end > p.start)) return tr('The end PA must be greater than the start PA.');
	if (pressureAdvanceTowerHeight(p) > 300)
		return tr('Tower too tall (> 300 mm): increase the step.');
	return null;
}
