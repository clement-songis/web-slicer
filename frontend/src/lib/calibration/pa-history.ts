// Historique des calibrations Pressure Advance (Orca : PACalibResult /
// CaliHistoryDialog), stocké côté serveur : /api/calibrations/pa (lot 14),
// filtres imprimante / filament / buse. L'ancien stockage navigateur
// (phase 19, avant la route) est migré une fois vers le serveur.
import { api } from '#lib/api/client';

export type PaMethod = 'pa_line' | 'pa_pattern' | 'pa_tower' | 'manual';
// Libellés = clés anglaises, traduites au rendu.
export const PA_METHODS: { id: PaMethod; label: string }[] = [
	{ id: 'pa_tower', label: 'Tower' },
	{ id: 'pa_line', label: 'Lines' },
	{ id: 'pa_pattern', label: 'Pattern' },
	{ id: 'manual', label: 'Manual entry' }
];

export interface PaRecord {
	id: string;
	printer_id?: string | null;
	machine_preset: string;
	filament_preset: string;
	filament_id?: string | null;
	nozzle_diameter: number;
	extruder_id: number;
	nozzle_volume_type?: string | null;
	k_value: number;
	n_coef: number;
	method: PaMethod;
	name: string;
	note?: string | null;
	created_at: string;
	updated_at: string;
}

export type PaInput = Omit<
	PaRecord,
	'id' | 'created_at' | 'updated_at' | 'extruder_id' | 'n_coef'
> & {
	extruder_id?: number;
	n_coef?: number;
};

export interface PaFilter {
	machine_preset?: string;
	filament_preset?: string;
	nozzle_diameter?: number;
	printer_id?: string;
}

export function paQuery(f: PaFilter): string {
	const q = new URLSearchParams();
	for (const [k, v] of Object.entries(f)) if (v !== undefined && v !== '') q.set(k, String(v));
	const s = q.toString();
	return s ? `?${s}` : '';
}

export const listPa = (f: PaFilter) => api.get<PaRecord[]>(`/calibrations/pa${paQuery(f)}`);
export const createPa = (body: PaInput) => api.post<PaRecord>('/calibrations/pa', body);
export const updatePa = (id: string, body: PaInput) =>
	api.put<PaRecord>(`/calibrations/pa/${id}`, body);
export const deletePa = (id: string) => api.del<void>(`/calibrations/pa/${id}`);

/** Corps PUT complet depuis un enregistrement existant. */
export function toInput(r: PaRecord, patch: Partial<PaInput> = {}): PaInput {
	return {
		printer_id: r.printer_id ?? null,
		machine_preset: r.machine_preset,
		filament_preset: r.filament_preset,
		filament_id: r.filament_id ?? null,
		nozzle_diameter: r.nozzle_diameter,
		extruder_id: r.extruder_id,
		nozzle_volume_type: r.nozzle_volume_type ?? null,
		k_value: r.k_value,
		n_coef: r.n_coef,
		method: r.method,
		name: r.name,
		note: r.note ?? null,
		...patch
	};
}

/** K valide (Orca : 0 ≤ K ≤ 2 ; 4 décimales). */
export function parseK(raw: string): number | null {
	const k = Number(raw.replace(',', '.'));
	return Number.isFinite(k) && k >= 0 && k <= 2 ? Math.round(k * 10000) / 10000 : null;
}

/** Plus récents d'abord, filament courant en tête. */
export function sortPa(list: PaRecord[], filament?: string): PaRecord[] {
	return [...list].sort(
		(a, b) =>
			Number(b.filament_preset === filament) - Number(a.filament_preset === filament) ||
			b.updated_at.localeCompare(a.updated_at)
	);
}

// — Migration de l'ancien stockage navigateur -------------------------------
export const LEGACY_KEY = 'web-slicer.pa-history';
interface LegacyEntry {
	printer: string;
	nozzle: string;
	filament: string;
	k: number;
	note?: string;
}
type Store = Pick<Storage, 'getItem' | 'removeItem'>;

/** Entrées locales à migrer (et les retire du navigateur). */
export function takeLegacy(store: Store | undefined): PaInput[] {
	try {
		const raw = JSON.parse(store?.getItem(LEGACY_KEY) ?? '[]') as LegacyEntry[];
		store?.removeItem(LEGACY_KEY);
		return Array.isArray(raw)
			? raw
					.filter((e) => e && Number.isFinite(e.k) && e.printer && e.filament)
					.map((e) => ({
						machine_preset: e.printer,
						filament_preset: e.filament,
						nozzle_diameter: Number(e.nozzle) || 0.4,
						k_value: e.k,
						method: 'manual' as const,
						name: e.filament,
						note: e.note ?? null
					}))
			: [];
	} catch {
		return [];
	}
}
