// Calcul des volumes de purge depuis les couleurs (Orca FlushVolCalculator,
// Sidebar::auto_calc_flushing_volumes) : route moteur proposée
// (phase15/BACKEND.md §4) — POST /api/flush-volumes { colours, multiplier }
// → { matrix } (NxN aplatie, mm³). Tant qu'elle n'existe pas, l'interface la
// détecte (404/405/501) et désactive le bouton au lieu d'échouer au clic.
import { ApiError, api } from './client';

export interface FlushVolumesResponse {
	matrix: number[];
	multiplier?: number;
}

/** `projectId` : le moteur lit aussi la config résolue du projet (types de
 *  filament, supports, purge minimale), comme Orca. */
export const computeFlushVolumes = (colours: string[], multiplier = 1, projectId?: string) =>
	api.post<FlushVolumesResponse>('/flush-volumes', {
		colours,
		multiplier,
		...(projectId ? { project_id: projectId } : {})
	});

let availability: Promise<boolean> | null = null;

/** Route présente ? Sondée une fois par session (corps vide : 400 attendu si
 *  la route existe ; 404/405/501 = absente). */
export function flushVolumesAvailable(): Promise<boolean> {
	availability ??= computeFlushVolumes([])
		.then(() => true)
		.catch((e) => !(e instanceof ApiError && [404, 405, 501].includes(e.status)));
	return availability;
}

/** Réinitialise la sonde (tests, redéploiement). */
export function resetFlushVolumesProbe(): void {
	availability = null;
}
