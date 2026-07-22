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
	/** Surcharges de réglages au niveau projet (clé → valeur au format Orca),
	 *  appliquées **par-dessus** la chaîne de presets au tranchage (T146). Permet
	 *  d'ajuster un réglage dans le panneau sans dériver un preset. */
	overrides: Record<string, string>;
}

/** Sélection vide (nouveau projet sans presets résolus). */
export function emptyActivePresets(): ActivePresets {
	return { printer: null, process: null, filaments: [], overrides: {} };
}

/** Lit le blob `active_presets` (JSON libre) en une sélection typée, robuste. */
export function parseActivePresets(raw: unknown): ActivePresets {
	const obj = raw && typeof raw === 'object' ? (raw as Record<string, unknown>) : {};
	const str = (v: unknown): string | null => (typeof v === 'string' && v.length > 0 ? v : null);
	const filaments = Array.isArray(obj.filaments)
		? obj.filaments.filter((v): v is string => typeof v === 'string' && v.length > 0)
		: [];
	const overrides: Record<string, string> = {};
	if (obj.overrides && typeof obj.overrides === 'object') {
		for (const [k, v] of Object.entries(obj.overrides as Record<string, unknown>)) {
			if (typeof v === 'string') overrides[k] = v;
		}
	}
	return { printer: str(obj.printer), process: str(obj.process), filaments, overrides };
}

/** Sérialise vers le blob `active_presets` attendu par le backend (clés omises si vides). */
export function serializeActivePresets(a: ActivePresets): Record<string, unknown> {
	const out: Record<string, unknown> = {};
	if (a.printer) out.printer = a.printer;
	if (a.process) out.process = a.process;
	if (a.filaments.length) out.filaments = a.filaments;
	if (Object.keys(a.overrides).length) out.overrides = a.overrides;
	return out;
}

/** Remplace la table de surcharges (immuable). */
export function setOverrides(a: ActivePresets, overrides: Record<string, string>): ActivePresets {
	return { ...a, overrides };
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
