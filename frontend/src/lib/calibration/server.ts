// Calibrations construites côté serveur (lot moteur 14 :
// POST /api/projects/{id}/calibration, portage libslic3r des Plater::calib_*).
// Comme Orca (le projet de calibration remplace le plateau courant sans
// toucher au projet d'origine), un NOUVEAU projet est créé avec les presets
// actifs, puis la calibration y est construite par le serveur.
import { api } from '$lib/api/client';
import { createProject, deleteProject } from '$lib/api/projects';
import { tr } from '$lib/i18n';
import type { EngineCalibKind, RangeParams } from './engine-calib';
import type { FlowPattern } from './flowrate';

/** Les 14 calibrations d'Orca portées côté serveur (CALIB_KINDS du moteur). */
export type ServerCalibKind =
	| 'pa_pattern'
	| 'is_freq'
	| 'is_damp'
	| 'cornering'
	| 'temp_tower'
	| 'pa_line'
	| 'pa_tower'
	| 'flow_pass1'
	| 'flow_pass2'
	| 'flow_yolo'
	| 'flow_yolo_fine'
	| 'max_vol_speed'
	| 'retraction'
	| 'vfa';

export const SERVER_CALIB_KINDS: readonly ServerCalibKind[] = [
	'pa_pattern',
	'is_freq',
	'is_damp',
	'cornering',
	'temp_tower',
	'pa_line',
	'pa_tower',
	'flow_pass1',
	'flow_pass2',
	'flow_yolo',
	'flow_yolo_fine',
	'max_vol_speed',
	'retraction',
	'vfa'
];

/** Corps de POST /api/projects/{id}/calibration (Calib_Params d'Orca). */
export interface CalibRequest {
	kind: ServerCalibKind;
	start?: number;
	end?: number;
	step?: number;
	/** Motif du test de débit : « archimedean » (défaut d'Orca) | « monotonic ». */
	option?: string;
	/** Motif PA : numéros imprimés, vitesses (mm/s) et accélérations (mm/s²). */
	print_numbers?: boolean;
	speeds?: number[];
	accelerations?: number[];
	/** Input shaping / cornering : 0 ringing_tower, 1 fast_tower_test, 2 SCV-V2. */
	test_model?: number;
	shaper_type?: string;
	/** Fréquences (Hz) : plage (test de fréquence) ou valeur fixe (amortissement). */
	freq_start_x?: number;
	freq_end_x?: number;
	freq_start_y?: number;
	freq_end_y?: number;
}

/** Tour de température (Orca : paliers de 5 °C). */
export const temperatureRequest = (p: { start: number; end: number }): CalibRequest => ({
	kind: 'temp_tower',
	start: p.start,
	end: p.end,
	step: 5
});

/** Pressure advance : tour (pa_tower) ou lignes (pa_line). */
export const pressureAdvanceRequest = (
	p: { start: number; end: number; step: number },
	method: 'tower' | 'line' = 'tower'
): CalibRequest => ({ kind: method === 'line' ? 'pa_line' : 'pa_tower', ...p });

/** Débit : passes 1/2 (Orca « Flow rate ») ou YOLO linéaire. */
export function flowRequest(pass: 1 | 2, linear: boolean, pattern: FlowPattern): CalibRequest {
	const kind: ServerCalibKind = linear
		? pass === 1
			? 'flow_yolo'
			: 'flow_yolo_fine'
		: pass === 1
			? 'flow_pass1'
			: 'flow_pass2';
	return { kind, option: pattern === 'monotonic' ? 'monotonic' : 'archimedean' };
}

/** Calibrations « mode moteur » (rétraction, VFA, débit max, motif PA, input
 *  shaping, virages) : paramètres du dialogue → requête serveur. */
export function engineRequest(kind: EngineCalibKind, p: RangeParams): CalibRequest {
	const range = { start: p.start, end: p.end, step: p.step };
	switch (kind) {
		case 'retraction':
			return { kind: 'retraction', ...range };
		case 'vfa':
			return { kind: 'vfa', ...range };
		case 'max_volumetric_speed':
			return { kind: 'max_vol_speed', ...range };
		case 'pa_pattern':
			return {
				kind: 'pa_pattern',
				...range,
				print_numbers: p.printNumbers ?? true,
				speeds: p.speeds ?? [],
				accelerations: p.accelerations ?? []
			};
		case 'input_shaping_freq': {
			// Amortissement fixe (start = end), plages de fréquence X / Y.
			const damp = p.fixed ?? 0;
			return {
				kind: 'is_freq',
				start: damp,
				end: damp,
				step: 1,
				test_model: p.testModel ?? 0,
				shaper_type: p.shaperType ?? 'Default',
				freq_start_x: p.start,
				freq_end_x: p.end,
				freq_start_y: p.startY ?? p.start,
				freq_end_y: p.endY ?? p.end
			};
		}
		case 'input_shaping_damp': {
			// Plage d'amortissement, fréquences X / Y fixes.
			const fx = p.fixed ?? 0;
			const fy = p.fixedY ?? fx;
			return {
				kind: 'is_damp',
				start: p.start,
				end: p.end,
				step: 1,
				test_model: p.testModel ?? 0,
				shaper_type: p.shaperType ?? 'Default',
				freq_start_x: fx,
				freq_end_x: fx,
				freq_start_y: fy,
				freq_end_y: fy
			};
		}
		case 'cornering':
			return {
				kind: 'cornering',
				start: p.start,
				end: p.end,
				step: 1,
				test_model: p.testModel ?? 0
			};
	}
}

export interface ServerCalibResult {
	/** Nom final du projet, rendu unique par le serveur (« Pressure Advance Test (2) »). */
	name: string;
	/** Nom brut du modèle d'Orca (lot 42 K1). */
	calibration_name?: string;
	version: number;
}

/**
 * Crée le projet de calibration ; renvoie son id et son nom. Lot 42 K1 : le
 * serveur nomme lui-même le projet (nom du modèle d'Orca, rendu unique comme
 * les projets) — plus de renommage côté client. Le nom de création n'est qu'un
 * libellé générique traduit, remplacé par le serveur ; le projet est retiré si
 * la construction de la calibration échoue (pas de projet vide orphelin).
 */
export async function createServerCalibration(
	request: CalibRequest,
	activePresets: unknown
): Promise<{ id: string; name: string }> {
	const project = await createProject({ name: tr('Calibration'), active_presets: activePresets });
	try {
		const res = await api.post<ServerCalibResult>(`/projects/${project.id}/calibration`, request);
		return { id: project.id, name: res.name || project.name };
	} catch (e) {
		await deleteProject(project.id).catch(() => undefined);
		throw e;
	}
}
