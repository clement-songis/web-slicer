// Lecture du résultat d'un projet de calibration (après impression) : Orca
// laisse l'utilisateur calculer la valeur (wiki « Calibration ») puis la
// recopier dans le filament. Ici, le projet garde ses paramètres
// (`scene.calibration`) : on convertit la mesure (hauteur, palier) en valeur
// et on propose de l'appliquer au filament / de l'enregistrer. Pur → testable.

/** Paramètres de calibration persistés dans la scène (sous-ensemble utile). */
export interface SceneCalibration {
	mode?: unknown;
	start?: unknown;
	end?: unknown;
	step?: unknown;
}

export type CalibrationReading =
	| {
			kind: 'pa_tower';
			/** K à la hauteur `h` (mm) : PA = début + pas × hauteur (tour d'Orca). */
			start: number;
			step: number;
			/** Hauteur utile de la tour (mm). */
			maxHeight: number;
	  }
	| {
			kind: 'pa_direct';
			/** Lignes / motif : la valeur K est imprimée à côté de chaque essai. */
			start: number;
			end: number;
	  }
	| {
			kind: 'temp_tower';
			/** Paliers de la tour (°C), du bas vers le haut. */
			temperatures: number[];
	  };

const num = (v: unknown): number | null => {
	const n = Number(v);
	return Number.isFinite(n) ? n : null;
};

/** Lecture proposée pour un projet de calibration (null : rien à lire). */
export function calibrationReading(
	cal: SceneCalibration | null | undefined
): CalibrationReading | null {
	if (!cal) return null;
	const start = num(cal.start);
	const end = num(cal.end);
	const step = num(cal.step);
	if (start === null || end === null || step === null || step <= 0) return null;
	switch (cal.mode) {
		case 'pa_tower':
			return { kind: 'pa_tower', start, step, maxHeight: Math.max(0, (end - start) / step) };
		case 'pa_line':
		case 'pa_pattern':
			return { kind: 'pa_direct', start, end };
		case 'temp_tower': {
			// Orca (temp_tower) : un palier de 10 mm par température, de « début »
			// (en bas) à « fin » (en haut), par pas de `step`.
			const temperatures: number[] = [];
			const dir = end >= start ? 1 : -1;
			for (let t = start; dir > 0 ? t <= end + 1e-9 : t >= end - 1e-9; t += dir * step)
				temperatures.push(Math.round(t));
			return temperatures.length ? { kind: 'temp_tower', temperatures } : null;
		}
		default:
			return null;
	}
}

/** K (4 décimales, comme parseK) pour une hauteur mesurée sur la tour. */
export function kAtHeight(
	r: { start: number; step: number; maxHeight: number },
	height: number
): number | null {
	if (!Number.isFinite(height) || height < 0 || height > r.maxHeight + 1e-9) return null;
	return Math.round((r.start + r.step * height) * 10000) / 10000;
}
