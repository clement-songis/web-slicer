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
	/** Presets filament par slot (slot `i` ↔ extrudeur `i + 1` des objets). */
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

/**
 * Change le preset filament du slot `index` (0 par défaut) (immuable). Un index
 * au-delà de la fin ajoute en queue (jamais de trou dans le tableau, que le
 * backend lirait comme un filament invalide) ; `null` retire le slot.
 */
export function setFilament(a: ActivePresets, id: string | null, index = 0): ActivePresets {
	const filaments = [...a.filaments];
	const at = Math.max(0, Math.trunc(index));
	if (id === null) {
		if (at < filaments.length) filaments.splice(at, 1);
	} else if (at >= filaments.length) {
		filaments.push(id);
	} else {
		filaments[at] = id;
	}
	return { ...a, filaments };
}

/** Filament principal (premier extrudeur), ou null. */
export function primaryFilament(a: ActivePresets): string | null {
	return a.filaments[0] ?? null;
}

// --- Multi-filament (slots) -------------------------------------------------
// Parité OrcaSlicer : la colonne « Filament » liste un slot par filament ; le
// slot `i` alimente l'extrudeur `i + 1` choisi dans la liste d'objets (`0` =
// auto / réglage global). Ajouter un slot clone le dernier filament ; retirer
// un slot renumérote les extrudeurs des objets pour garder l'association.

/** Nombre de slots affichés : au moins un (sélecteur vide d'un nouveau projet). */
export function filamentSlotCount(a: ActivePresets): number {
	return Math.max(1, a.filaments.length);
}

/** Preset du slot `slot`, ou null (slot vide / hors bornes). */
export function filamentAt(a: ActivePresets, slot: number): string | null {
	return a.filaments[slot] ?? null;
}

/** Ramène un index de slot dans les bornes des slots affichés. */
export function clampFilamentSlot(a: ActivePresets, slot: number): number {
	const max = filamentSlotCount(a) - 1;
	if (!Number.isFinite(slot)) return 0;
	return Math.min(max, Math.max(0, Math.trunc(slot)));
}

/**
 * Ajoute un slot filament (immuable) : copie du dernier filament, sinon
 * `fallback` (premier preset compatible). Sans source, la sélection est
 * renvoyée telle quelle (pas de slot vide persistable).
 */
export function addFilamentSlot(a: ActivePresets, fallback: string | null = null): ActivePresets {
	const id = a.filaments[a.filaments.length - 1] ?? fallback;
	if (!id) return a;
	return { ...a, filaments: [...a.filaments, id] };
}

/**
 * Slots filament d'un projet 3MF ouvert (Orca load_project : un slot par
 * entrée de `filament_settings_id`). Le backend ne fixe que le process importé
 * (« configuration 3MF ») ; les couleurs du projet (`scene.projectConfig`)
 * donnent le nombre de filaments et `filament_settings_id` du process importé
 * leurs presets, retrouvés par nom (sinon : dernier filament, puis `fallback`).
 * Les slots existants sont conservés. `null` = rien à compléter.
 */
export function importedFilamentSlots(
	a: ActivePresets,
	colourCount: number,
	names: readonly unknown[],
	presets: readonly { id: string; name: string }[],
	fallback: string | null = null
): ActivePresets | null {
	if (!(colourCount > a.filaments.length)) return null;
	const byName = new Map(presets.map((p) => [p.name, p.id]));
	const named = (i: number) => {
		const n = names[i];
		return typeof n === 'string' ? (byName.get(n) ?? null) : null;
	};
	const filaments = [...a.filaments];
	for (let i = filaments.length; i < colourCount; i++) {
		const id = named(i) ?? filaments[filaments.length - 1] ?? named(0) ?? fallback;
		if (!id) return filaments.length > a.filaments.length ? { ...a, filaments } : null;
		filaments.push(id);
	}
	return { ...a, filaments };
}

/** Un slot peut être retiré s'il en reste au moins un autre. */
export function canRemoveFilamentSlot(a: ActivePresets, slot: number): boolean {
	return a.filaments.length > 1 && slot >= 0 && slot < a.filaments.length;
}

/** Retire le slot `slot` (immuable), sauf le dernier slot restant. */
export function removeFilamentSlot(a: ActivePresets, slot: number): ActivePresets {
	if (!canRemoveFilamentSlot(a, slot)) return a;
	return { ...a, filaments: a.filaments.filter((_, i) => i !== slot) };
}

/**
 * Extrudeur (1-based, `0` = auto) d'un objet après retrait du slot `removed`
 * (0-based) : les objets du slot retiré repassent en auto, ceux des slots
 * suivants descendent d'un cran pour rester sur le même filament.
 */
export function remapExtruderAfterRemoval(extruder: number, removed: number): number {
	if (extruder <= 0) return 0;
	const slot = extruder - 1;
	if (slot === removed) return 0;
	return slot > removed ? extruder - 1 : extruder;
}

/** Extrudeur hors des slots existants → auto (`0`) ; sinon inchangé. */
export function clampExtruder(extruder: number, slotCount: number): number {
	return extruder > 0 && extruder <= slotCount ? extruder : 0;
}

/** Slot sélectionné après retrait du slot `removed` (reste sur le même filament si possible). */
export function slotAfterRemoval(selected: number, removed: number, nextCount: number): number {
	const next = selected > removed ? selected - 1 : selected;
	return Math.min(Math.max(0, nextCount - 1), Math.max(0, next));
}

// --- Copies de presets ------------------------------------------------------

/**
 * Nom de la `attempt`-ième copie d'un preset, comme Orca : « X (copie) », puis
 * « X (copie 2) », « X (copie 3) »… (`attempt` commence à 1).
 */
export function copyName(base: string, attempt = 1): string {
	return attempt <= 1 ? `${base} (copie)` : `${base} (copie ${attempt})`;
}

/** Premier nom de copie libre parmi `taken` (noms déjà utilisés du même type). */
export function uniqueCopyName(base: string, taken: Iterable<string>): string {
	const used = new Set(taken);
	for (let n = 1; ; n++) {
		const name = copyName(base, n);
		if (!used.has(name)) return name;
	}
}
