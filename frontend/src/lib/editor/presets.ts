// Orchestrateur pur des presets actifs de l'éditeur (T098) : sélection courante
// par type (imprimante / filament·s / process) et (dé)sérialisation vers le blob
// `active_presets` du projet. Le schéma reflète le backend
// (`resolve_active_config` : clés `printer`, `process`, tableau `filaments`).
// Découplé de Svelte → testable sous bun ; le composant délègue chaque
// transition ici et confie dériver/enregistrer/supprimer à l'API presets.

/** Type de preset adressé par un sélecteur de la colonne de configuration. */
export type PresetKind = 'machine' | 'filament' | 'process';

/** Sélection de presets actifs d'un projet (miroir du blob `active_presets`). */
export interface ActivePresets {
	/** Preset machine (imprimante). */
	printer: string | null;
	/** Preset process (« Traitement »). */
	process: string | null;
	/** Presets filament par extrudeur (v1 mono-extrudeur : un seul). */
	filaments: string[];
}

/** Sélection vide (nouveau projet sans presets résolus). */
export function emptyActivePresets(): ActivePresets {
	return { printer: null, process: null, filaments: [] };
}

/** Lit le blob `active_presets` (JSON libre) en une sélection typée, robuste. */
export function parseActivePresets(raw: unknown): ActivePresets {
	const obj = raw && typeof raw === 'object' ? (raw as Record<string, unknown>) : {};
	const str = (v: unknown): string | null => (typeof v === 'string' && v.length > 0 ? v : null);
	const filaments = Array.isArray(obj.filaments)
		? obj.filaments.filter((v): v is string => typeof v === 'string' && v.length > 0)
		: [];
	return { printer: str(obj.printer), process: str(obj.process), filaments };
}

/** Sérialise vers le blob `active_presets` attendu par le backend (clés omises si vides). */
export function serializeActivePresets(a: ActivePresets): Record<string, unknown> {
	const out: Record<string, unknown> = {};
	if (a.printer) out.printer = a.printer;
	if (a.process) out.process = a.process;
	if (a.filaments.length) out.filaments = a.filaments;
	return out;
}

/** Change le preset machine (immuable). */
export function setPrinter(a: ActivePresets, id: string | null): ActivePresets {
	return { ...a, printer: id };
}

/** Change le preset process (immuable). */
export function setProcess(a: ActivePresets, id: string | null): ActivePresets {
	return { ...a, process: id };
}

/** Change le preset filament de l'extrudeur `index` (0 par défaut) (immuable). */
export function setFilament(a: ActivePresets, id: string | null, index = 0): ActivePresets {
	const filaments = [...a.filaments];
	if (id === null) {
		if (index < filaments.length) filaments.splice(index, 1);
	} else {
		filaments[index] = id;
	}
	return { ...a, filaments };
}

/** Filament principal (premier extrudeur), ou null. */
export function primaryFilament(a: ActivePresets): string | null {
	return a.filaments[0] ?? null;
}
