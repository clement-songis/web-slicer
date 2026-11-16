// Création d'un projet de calibration prêt à trancher (menu « Calibration »).
// Reproduit Plater::calib_temp d'Orca : nouveau projet, modèle de
// resources/calib, découpe haut/bas de la tour selon la plage demandée,
// surcharges de réglages. Les surcharges « objet » d'Orca sont portées par le
// preset process dérivé (projet mono-objet : effet identique) et la scène est
// laissée vide : le modèle final est restauré comme import (posé, centré) par
// l'éditeur à l'ouverture (modèle « orphelin », cf. scene/document).
import { PARAMS } from '../../generated/params';
import type { ActivePresets } from '$lib/editor/presets';
import {
	pressureAdvanceTowerHeight,
	type CalibrationPlan,
	type PressureAdvanceParams,
	type TemperatureParams
} from './calibration';

/** Constantes d'Orca (Plater::calib_temp). */
export const TEMP_TOWER = { blockHeight: 10, step: 5, eps: 1e-3 };

/** Hauteurs de découpe (mm depuis le bas du modèle), comme Orca : on garde
 *  `(500 − end)/5 + 1` blocs en partant du bas, puis on retire les
 *  `(500 − start)/5` premiers. Une découpe inutile (au-delà du modèle) est omise. */
export function temperatureTowerCuts(
	p: TemperatureParams,
	sizeZ: number
): { keepLowerBelow: number | null; keepUpperAbove: number | null } {
	const { blockHeight, step, eps } = TEMP_TOWER;
	const top = Math.round((500 - p.end) / step + 1) * blockHeight - eps;
	const bottomBlocks = Math.round((500 - p.start) / step);
	const bottom = bottomBlocks * blockHeight + eps;
	return {
		keepLowerBelow: top > 0 && top < sizeZ ? top : null,
		keepUpperAbove: bottomBlocks > 0 && bottom < Math.min(sizeZ, top) ? bottom : null
	};
}

/** Étendue Z d'un maillage (positions brutes, repère du modèle). */
export function zExtent(positions: Float32Array): { min: number; max: number } {
	let min = Infinity;
	let max = -Infinity;
	for (let i = 2; i < positions.length; i += 3) {
		if (positions[i] < min) min = positions[i];
		if (positions[i] > max) max = positions[i];
	}
	return { min, max };
}

/** Converti une hauteur depuis le bas en Z du repère recentré (API /cut). */
export function recenteredZ(fromBottom: number, ext: { min: number; max: number }): number {
	return ext.min + fromBottom - (ext.min + ext.max) / 2;
}

/** Ne garde que les clés connues du registre (un preset invalide serait refusé). */
export function knownKeys(values: Record<string, unknown>): Record<string, unknown> {
	return Object.fromEntries(Object.entries(values).filter(([k]) => k in PARAMS));
}

/** Découpe de la tour PA (Plater::_calib_pa_tower) : garde le bas jusqu'à
 *  `ceil((end − start)/step) + 1` mm (1 mm par incrément de PA). */
export function pressureAdvanceTowerCuts(
	p: PressureAdvanceParams,
	sizeZ: number
): { keepLowerBelow: number | null; keepUpperAbove: number | null } {
	const h = pressureAdvanceTowerHeight(p);
	return { keepLowerBelow: h < sizeZ ? h : null, keepUpperAbove: null };
}

export type TowerCuts = (sizeZ: number) => {
	keepLowerBelow: number | null;
	keepUpperAbove: number | null;
};

export interface PresetRef {
	id: string;
	name: string;
}

export interface CalibrationDeps {
	createPreset(
		kind: 'machine' | 'filament' | 'process',
		base: string,
		inherits: string,
		values: Record<string, unknown>
	): Promise<PresetRef>;
	createProject(name: string, active: ActivePresets): Promise<{ id: string }>;
	fetchModelFile(path: string): Promise<File>;
	uploadModel(projectId: string, file: File): Promise<{ id: string }>;
	/** Étendue Z du maillage converti (attend la fin de conversion). */
	modelZExtent(modelId: string): Promise<{ min: number; max: number }>;
	/** Largeur X du maillage converti (optionnel : mise à l'échelle sur plateau étroit). */
	modelXSize?(modelId: string): Promise<number>;
	cut(modelId: string, z: number, keep: 'upper' | 'lower'): Promise<{ id: string }>;
	saveScene(projectId: string, scene: unknown): Promise<void>;
	/** Annulation (rollback) en cas d'échec : suppression best-effort. */
	deletePreset(id: string): Promise<void>;
	deleteProject(id: string): Promise<void>;
	/** Centre du plateau (mm) où poser la tour. */
	bedCenter(): [number, number];
}

/** Plater::calib_max_vol_speed : `scale_obj = (bed_x − 10) / size_x` si < 1. */
async function fitScaleX(
	plan: CalibrationPlan,
	deps: CalibrationDeps,
	modelId: string
): Promise<number> {
	if (!plan.fitWidth || !deps.modelXSize) return 1;
	const size = await deps.modelXSize(modelId);
	const scale = size > 0 ? (plan.fitWidth - 10) / size : 1;
	return scale < 1 ? scale : 1;
}

export interface CalibrationSource {
	printer: PresetRef;
	process: PresetRef;
	filament: PresetRef;
}

/** Libellé court des presets dérivés. */
const SUFFIX: Record<CalibrationPlan['kind'], string> = {
	temperature: 'Calibration température',
	pressure_advance: 'Calibration PA',
	retraction: 'Calibration rétraction',
	vfa: 'Calibration VFA',
	max_volumetric_speed: 'Calibration débit max',
	pa_pattern: 'Calibration PA (motif)',
	input_shaping_freq: 'Calibration input shaping (fréquence)',
	input_shaping_damp: 'Calibration input shaping (amortissement)',
	cornering: 'Calibration virages'
};

/**
 * Orchestration : 3 presets dérivés (machine, filament, process) → projet →
 * import du modèle → découpes → scène (modèle final à restaurer). Renvoie
 * l'id du projet créé. Tout échec annule ce qui a été créé (presets, projet —
 * les modèles importés partent avec le projet) puis relance l'erreur.
 */
export async function createCalibrationProject(
	plan: CalibrationPlan,
	source: CalibrationSource,
	deps: CalibrationDeps,
	cutsFor: TowerCuts
): Promise<string> {
	const suffix = SUFFIX[plan.kind];
	const presets: string[] = [];
	let projectId: string | null = null;
	try {
		const printer = await deps.createPreset(
			'machine',
			`${source.printer.name} — ${suffix}`,
			source.printer.name,
			knownKeys(plan.printer)
		);
		presets.push(printer.id);
		const filament = await deps.createPreset(
			'filament',
			`${source.filament.name} — ${suffix}`,
			source.filament.name,
			knownKeys(plan.filament)
		);
		presets.push(filament.id);
		const process = await deps.createPreset(
			'process',
			`${source.process.name} — ${suffix}`,
			source.process.name,
			knownKeys({ ...plan.objectSettings, ...plan.process })
		);
		presets.push(process.id);
		const project = await deps.createProject(plan.projectName, {
			printer: printer.id,
			process: process.id,
			filaments: [filament.id]
		});
		projectId = project.id;

		const file = await deps.fetchModelFile(plan.model);
		const original = await deps.uploadModel(project.id, file);
		const intermediate: string[] = [original.id];
		let current = original.id;

		const ext = await deps.modelZExtent(current);
		const cuts = cutsFor(ext.max - ext.min);
		if (cuts.keepLowerBelow !== null) {
			current = (await deps.cut(current, recenteredZ(cuts.keepLowerBelow, ext), 'lower')).id;
			intermediate.push(current);
		}
		if (cuts.keepUpperAbove !== null) {
			const ext2 = await deps.modelZExtent(current);
			current = (await deps.cut(current, recenteredZ(cuts.keepUpperAbove, ext2), 'upper')).id;
			intermediate.push(current);
		}

		// Scène explicite : un seul objet (modèle final) posé au centre du plateau ;
		// tous les modèles du projet sont « connus » → les intermédiaires (tour
		// complète, moitiés) ne sont ni restaurés ni tranchés.
		const finalExt = current === original.id ? ext : await deps.modelZExtent(current);
		const [cx, cy] = deps.bedCenter();
		await deps.saveScene(project.id, {
			schemaVersion: 1,
			plates: {
				plates: [{ id: 'plate-1', name: 'Plateau 1', plateType: '', objectIds: ['calib-1'] }],
				activeId: 'plate-1'
			},
			objects: [
				{
					id: 'calib-1',
					modelId: current,
					name: plan.projectName,
					transform: {
						position: [cx, cy, (finalExt.max - finalExt.min) / 2],
						rotation: [0, 0, 0],
						scale: [await fitScaleX(plan, deps, current), 1, 1]
					},
					extruder: 0,
					settings: {},
					painting: {},
					layerProfile: [],
					brimEars: { points: [] }
				}
			],
			knownModels: { projectId: project.id, ids: intermediate },
			...(plan.calibration ? { calibration: plan.calibration } : {})
		});
		return project.id;
	} catch (e) {
		if (projectId) await deps.deleteProject(projectId).catch(() => {});
		for (const id of presets.reverse()) await deps.deletePreset(id).catch(() => {});
		throw e;
	}
}
