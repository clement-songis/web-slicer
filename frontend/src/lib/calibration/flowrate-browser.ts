// Création navigateur du projet de calibration du débit (passes 1/2) : presets
// dérivés, import du 3MF d'Orca (géométrie seule, comme Plater::add_model :
// le preset embarqué créé par l'import est supprimé), réglages par objet.
import { deletePreset, getResolvedPreset } from '$lib/api/presets';
import { getProject, importProject, listProjectModels, saveProject } from '$lib/api/projects';
import { serializeActivePresets } from '$lib/editor/presets';
import { browserCalibrationDeps } from './browser';
import type { CalibrationSource } from './create';
import { knownKeys } from './create';
import {
	applyFlowPlanToScene,
	planFlowRate,
	type FlowContext,
	type FlowOptions,
	type FlowPass
} from './flowrate';

const first = (v: unknown) => Number(Array.isArray(v) ? v[0] : v);
const num = (v: unknown, fallback: number) => {
	const n = first(v);
	return Number.isFinite(n) && n > 0 ? n : fallback;
};

export async function flowContext(source: CalibrationSource): Promise<FlowContext> {
	const [printer, process, filament] = await Promise.all([
		getResolvedPreset(source.printer.id),
		getResolvedPreset(source.process.id),
		getResolvedPreset(source.filament.id)
	]);
	const p = printer.values as Record<string, unknown>;
	const q = process.values as Record<string, unknown>;
	const f = filament.values as Record<string, unknown>;
	return {
		nozzleDiameter: num(p.nozzle_diameter, 0.4),
		initialLayerHeight: num(q.initial_layer_print_height, 0.2),
		internalSolidInfillSpeed: num(q.internal_solid_infill_speed, 0),
		topSurfaceSpeed: num(q.top_surface_speed, 0),
		filamentMaxVolumetricSpeed: num(f.filament_max_volumetric_speed, 12),
		filamentFlowRatio: num(f.filament_flow_ratio, 1)
	};
}

/** Crée le projet de calibration du débit ; renvoie son id. */
export async function createFlowRateProject(
	pass: FlowPass,
	source: CalibrationSource,
	takenNames: string[],
	options: FlowOptions = {}
): Promise<string> {
	const plan = planFlowRate(pass, await flowContext(source), options);
	const deps = browserCalibrationDeps(takenNames);
	const created: string[] = [];
	let projectId: string | null = null;
	try {
		const suffix = options.linear
			? `Calibration YOLO débit${pass === 2 ? ' (perfectionniste)' : ''}`
			: `Calibration débit (passe ${pass})`;
		const printer = await deps.createPreset(
			'machine',
			`${source.printer.name} — ${suffix}`,
			source.printer.name,
			knownKeys(plan.printer)
		);
		created.push(printer.id);
		const process = await deps.createPreset(
			'process',
			`${source.process.name} — ${suffix}`,
			source.process.name,
			knownKeys(plan.process)
		);
		created.push(process.id);

		const file = await deps.fetchModelFile(plan.model);
		const imported = await importProject(file);
		projectId = imported.id;
		const embedded = (imported.active_presets as { process?: string } | null)?.process;

		const project = await getProject(imported.id);
		const models = await listProjectModels(imported.id);
		const names = new Map(models.map((m) => [m.id, m.filename]));
		const { scene, tuned } = applyFlowPlanToScene(project.scene as { objects?: [] }, plan, (id) =>
			names.get(id)
		);
		if (tuned === 0) throw new Error('aucun objet « flowrate_* » dans le modèle de calibration');
		const active = serializeActivePresets({
			printer: printer.id,
			process: process.id,
			filaments: [source.filament.id]
		});
		await saveProject(
			imported.id,
			Number(project.version),
			{ ...scene, knownModels: { projectId: imported.id, ids: models.map((m) => m.id) } },
			active
		);
		// Le preset embarqué du 3MF n'est plus référencé (Orca n'importe que la géométrie).
		if (embedded) await deletePreset(embedded).catch(() => {});
		return imported.id;
	} catch (e) {
		if (projectId) await deps.deleteProject(projectId).catch(() => {});
		for (const id of created.reverse()) await deps.deletePreset(id).catch(() => {});
		throw e;
	}
}
