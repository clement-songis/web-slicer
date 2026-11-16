// Branchement navigateur de la création de calibration : implémente
// `CalibrationDeps` avec les API existantes (presets, projets, modèles, cut).
import { ApiError } from '$lib/api/client';
import { createPreset, deletePreset, getResolvedPreset } from '$lib/api/presets';
import { createProject, deleteProject, saveProject } from '$lib/api/projects';
import { cutModel } from '$lib/api/scene';
import { uniqueCopyName, serializeActivePresets } from '$lib/editor/presets';
import { fetchMesh, uploadModel } from '$lib/scene/loaders';
import type { CalibrationContext } from './calibration';
import { zExtent, type CalibrationDeps } from './create';

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** Contexte imprimante (firmware, buse, G-code de couche) depuis le preset résolu. */
export async function calibrationContext(printerPresetId: string): Promise<CalibrationContext> {
	const { values } = await getResolvedPreset(printerPresetId);
	const v = values as Record<string, unknown>;
	const nozzle = Array.isArray(v.nozzle_diameter)
		? Number(v.nozzle_diameter[0])
		: Number(v.nozzle_diameter);
	return {
		gcodeFlavor: String(v.gcode_flavor ?? 'marlin'),
		nozzleDiameter: Number.isFinite(nozzle) && nozzle > 0 ? nozzle : 0.4,
		layerChangeGcode: typeof v.layer_change_gcode === 'string' ? v.layer_change_gcode : '',
		bedCenter: bedCenterOf(v.printable_area)
	};
}

/** Centre de printable_area (« XxY » par sommet) ; défaut 110×110. */
export function bedCenterOf(area: unknown): [number, number] {
	const pts = (Array.isArray(area) ? area : typeof area === 'string' ? area.split(',') : [])
		.map((s) => String(s).split('x').map(Number))
		.filter((p) => p.length === 2 && p.every(Number.isFinite));
	if (pts.length === 0) return [110, 110];
	const xs = pts.map((p) => p[0]);
	const ys = pts.map((p) => p[1]);
	return [(Math.min(...xs) + Math.max(...xs)) / 2, (Math.min(...ys) + Math.max(...ys)) / 2];
}

export function browserCalibrationDeps(
	takenNames: string[],
	bedCenter: [number, number] = [110, 110]
): CalibrationDeps {
	const taken = new Set(takenNames);
	let version = 0;
	let activeSaved: Record<string, unknown> = {};
	return {
		async createPreset(kind, base, inherits, values) {
			for (let attempt = 0; attempt < 50; attempt++) {
				const name = attempt === 0 && !taken.has(base) ? base : uniqueCopyName(base, taken);
				try {
					const p = await createPreset({ kind, name, inherits, values });
					taken.add(name);
					return { id: p.id, name: p.name };
				} catch (e) {
					if (!(e instanceof ApiError && e.status === 409)) throw e;
					taken.add(name);
				}
			}
			throw new Error('nom de preset indisponible');
		},
		async createProject(name, active) {
			activeSaved = serializeActivePresets(active);
			const p = await createProject({ name, active_presets: activeSaved });
			version = Number(p.version);
			return { id: p.id };
		},
		async fetchModelFile(path) {
			const res = await fetch('/' + path);
			if (!res.ok)
				throw new Error(
					`modèle de calibration introuvable (/${path}) : lancez scripts/copy-calib-models.mjs`
				);
			return new File([await res.blob()], path.split('/').pop() ?? 'calib.drc');
		},
		async uploadModel(projectId, file) {
			const m = await uploadModel(projectId, file);
			return { id: m.id };
		},
		async modelZExtent(modelId) {
			// Conversion moteur asynchrone : 409 tant que le maillage n'est pas prêt.
			for (let i = 0; i < 120; i++) {
				try {
					return zExtent((await fetchMesh(modelId)).positions);
				} catch (e) {
					if (!(e instanceof ApiError && (e.status === 409 || e.status === 404))) throw e;
					await sleep(1000);
				}
			}
			throw new Error('conversion du modèle de calibration trop longue');
		},
		async modelXSize(modelId) {
			const p = (await fetchMesh(modelId)).positions;
			let min = Infinity;
			let max = -Infinity;
			for (let i = 0; i < p.length; i += 3) {
				if (p[i] < min) min = p[i];
				if (p[i] > max) max = p[i];
			}
			return max > min ? max - min : 0;
		},
		async cut(modelId, z, keep) {
			const res = await cutModel(modelId, {
				plane: { point: [0, 0, z], normal: [0, 0, 1] },
				keep_upper: keep === 'upper',
				keep_lower: keep === 'lower'
			});
			const part = res.parts.find((p) => p.label === keep) ?? res.parts[0];
			if (!part) throw new Error('découpe sans résultat');
			return { id: part.model.id };
		},
		bedCenter: () => bedCenter,
		async deletePreset(id) {
			await deletePreset(id);
		},
		async deleteProject(id) {
			await deleteProject(id);
		},
		async saveScene(projectId, scene) {
			const p = await saveProject(projectId, version, scene, activeSaved);
			version = Number(p.version);
		}
	};
}
