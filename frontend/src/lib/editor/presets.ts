// Orchestrateur pur des presets actifs de l'éditeur (T098) : sélection courante
// par type (imprimante / filament·s / process) et (dé)sérialisation vers le blob
// `active_presets` du projet. Le schéma reflète le backend
// (`resolve_active_config` : clés `printer`, `process`, tableau `filaments`).
// Découplé de Svelte → testable sous bun ; le composant délègue chaque
// transition ici et confie dériver/enregistrer/supprimer à l'API presets.

import type { SliceDraft } from '../../generated/api/SliceDraft';

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
	/**
	 * Lot 40 (D1) : réglages modifiés **sans être enregistrés** en preset
	 * (preset « (modifié) » d'Orca), par type de preset, forme `SliceDraft`.
	 * Absent ⇒ aucune modification.
	 */
	overrides?: SliceDraft;
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
	const out: ActivePresets = { printer: str(obj.printer), process: str(obj.process), filaments };
	const overrides = compactOverrides(obj.overrides);
	if (overrides) out.overrides = overrides;
	return out;
}

/**
 * Sérialise vers le blob `active_presets` attendu par le backend (clés omises
 * si vides). `overrides` est compacté (comme le serveur) : omis s'il est vide
 * — le document remplace celui du projet en bloc, l'omettre efface donc les
 * modifications enregistrées.
 */
export function serializeActivePresets(a: ActivePresets): Record<string, unknown> {
	const out: Record<string, unknown> = {};
	if (a.printer) out.printer = a.printer;
	if (a.process) out.process = a.process;
	if (a.filaments.length) out.filaments = a.filaments;
	const overrides = compactOverrides(a.overrides);
	if (overrides) out.overrides = overrides;
	return out;
}

// --- Surcharges non enregistrées (lot 40, D1) ---------------------------------

type Values = Record<string, unknown>;

const isRecord = (v: unknown): v is Values => !!v && typeof v === 'object' && !Array.isArray(v);

/** Copie à clés triées (empreinte stable), ou `null` si vide / illisible. */
function compactValues(v: unknown): Values | null {
	if (!isRecord(v)) return null;
	const keys = Object.keys(v)
		.filter((k) => v[k] !== undefined)
		.sort();
	if (!keys.length) return null;
	return Object.fromEntries(keys.map((k) => [k, v[k]]));
}

/**
 * Forme canonique d'un document `overrides` (même compactage que le serveur) :
 * objets vides retirés, filaments vides → `null`, `null` de fin tronqués, clés
 * triées. `undefined` = aucune modification.
 */
export function compactOverrides(raw: unknown): SliceDraft | undefined {
	if (!isRecord(raw)) return undefined;
	const out: SliceDraft = {};
	const process = compactValues(raw.process);
	const printer = compactValues(raw.printer);
	if (process) out.process = process;
	if (printer) out.printer = printer;
	if (Array.isArray(raw.filaments)) {
		const filaments = raw.filaments.map((f) => compactValues(f));
		while (filaments.length && filaments[filaments.length - 1] === null) filaments.pop();
		if (filaments.length) out.filaments = filaments;
	}
	return Object.keys(out).length ? out : undefined;
}

/** Type de preset au sens de `SliceDraft` (« printer » = machine). */
export type OverrideKind = 'process' | 'filament' | 'machine';

/**
 * Surcharges d'un type (filament : slot `slot`), ou `null` si aucune —
 * base du badge « (modifié) » tant que le preset n'est pas chargé.
 */
export function overridesOf(
	o: SliceDraft | undefined,
	kind: OverrideKind,
	slot = 0
): Values | null {
	if (!o) return null;
	const v = kind === 'process' ? o.process : kind === 'machine' ? o.printer : o.filaments?.[slot];
	return compactValues(v);
}

/**
 * Retire des surcharges les clés `keys` du type `kind` (filament : slot
 * `slot`) — toutes si `keys` est omis : « Enregistrer / Enregistrer sous » (le
 * preset porte désormais ces valeurs) et « Abandonner » (entrée du type
 * retirée). Immuable, résultat compacté.
 */
export function withoutOverrides(
	o: SliceDraft | undefined,
	kind: OverrideKind,
	slot = 0,
	keys?: Iterable<string>
): SliceDraft | undefined {
	if (!o) return undefined;
	const drop = (v: Values | null | undefined): Values | null => {
		if (!v || !keys) return null;
		const next = { ...v };
		for (const k of keys) delete next[k];
		return next;
	};
	const next: SliceDraft = { ...o };
	if (kind === 'process') next.process = drop(o.process) ?? undefined;
	else if (kind === 'machine') next.printer = drop(o.printer) ?? undefined;
	else if (o.filaments) {
		next.filaments = o.filaments.map((f, i) => (i === slot ? drop(f) : f));
	}
	return compactOverrides(next);
}

/** Surcharges filament après retrait du slot `slot` (les suivants descendent d'un cran). */
export function overridesAfterSlotRemoval(
	o: SliceDraft | undefined,
	slot: number
): SliceDraft | undefined {
	if (!o?.filaments) return compactOverrides(o);
	return compactOverrides({ ...o, filaments: o.filaments.filter((_, i) => i !== slot) });
}

/**
 * Valeur de surcharge mise au type de la valeur résolue du preset : un 3MF
 * Orca peut porter des chaînes (« 5 », « 1 », « 0.4,0.4 ») là où
 * `/resolved` donne du JSON typé (5, true, [0.4, 0.4]) — le widget du dialogue
 * attend la forme résolue. Autres cas : valeur inchangée.
 */
export function coerceOverrideValue(base: unknown, value: unknown): unknown {
	if (typeof value !== 'string') return value;
	const s = value.trim();
	const num = (t: string) => (t !== '' && Number.isFinite(Number(t)) ? Number(t) : null);
	const bool = (t: string) =>
		t === '1' || t === 'true' ? true : t === '0' || t === 'false' ? false : null;
	if (typeof base === 'number') return num(s) ?? value;
	if (typeof base === 'boolean') return bool(s) ?? value;
	if (Array.isArray(base) && base.length > 0) {
		const parts = s.split(',').map((t) => t.trim());
		if (typeof base[0] === 'number') {
			const nums = parts.map(num);
			return nums.every((n) => n !== null) ? nums : value;
		}
		if (typeof base[0] === 'boolean') {
			const bools = parts.map(bool);
			return bools.every((b) => b !== null) ? bools : value;
		}
	}
	return value;
}

// Surcharges « en attente » : document `ActivePresets` dont les ids disent à
// quel preset s'applique chaque entrée de `overrides` (projet ouvert, slots
// filament non affichés). Une entrée n'est reprise que si le preset du type
// (du slot) est toujours celui-là ; elle est consommée au chargement du
// brouillon correspondant.

/** Id du preset auquel s'applique l'entrée `kind` (`slot`) d'un document. */
function presetIdOf(a: ActivePresets, kind: OverrideKind, slot: number): string | null {
	if (kind === 'process') return a.process;
	if (kind === 'machine') return a.printer;
	return a.filaments[slot] ?? null;
}

/** Surcharges en attente pour le preset `presetId` du type `kind` (`slot`), ou null. */
export function pendingOverridesFor(
	pending: ActivePresets,
	kind: OverrideKind,
	slot: number,
	presetId: string | null
): Values | null {
	if (!presetId || presetIdOf(pending, kind, slot) !== presetId) return null;
	return overridesOf(pending.overrides, kind, slot);
}

/** Retire l'entrée en attente du type `kind` (`slot`) (immuable). */
export function withoutPending(
	pending: ActivePresets,
	kind: OverrideKind,
	slot = 0
): ActivePresets {
	if (!overridesOf(pending.overrides, kind, slot)) return pending;
	return { ...pending, overrides: withoutOverrides(pending.overrides, kind, slot) };
}

/**
 * Met en attente les modifications du slot filament `slot` (preset
 * `presetId`) quand le panneau passe à un autre slot (immuable).
 */
export function withPendingFilament(
	pending: ActivePresets,
	slot: number,
	presetId: string,
	values: Values
): ActivePresets {
	const filaments = [...pending.filaments];
	while (filaments.length <= slot) filaments.push('');
	filaments[slot] = presetId;
	const entries: Array<Values | null> = [...(pending.overrides?.filaments ?? [])];
	while (entries.length <= slot) entries.push(null);
	entries[slot] = values;
	return {
		...pending,
		filaments,
		overrides: compactOverrides({ ...pending.overrides, filaments: entries })
	};
}

/** Surcharges en attente après retrait du slot filament `slot` (immuable). */
export function pendingAfterSlotRemoval(pending: ActivePresets, slot: number): ActivePresets {
	return {
		...pending,
		filaments: pending.filaments.filter((_, i) => i !== slot),
		overrides: overridesAfterSlotRemoval(pending.overrides, slot)
	};
}

/** Brouillon d'un preset (`resolved` complété) + surcharges enregistrées, typées comme la base. */
export function applyOverrides(resolved: Values, overrides: Values | null): Values {
	if (!overrides) return resolved;
	const next = { ...resolved };
	for (const [k, v] of Object.entries(overrides)) next[k] = coerceOverrideValue(resolved[k], v);
	return next;
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

/**
 * « Fusionner avec » (Orca create_filament_action_menu → Sidebar::change_filament
 * → delete_filament(from, to)) : extrudeur (1-based, `0` = auto) après la
 * fusion du slot `removed` dans le slot `target` (0-based, numérotation
 * d'avant) — les objets du slot retiré passent au filament cible, les slots
 * suivants descendent d'un cran.
 */
export function remapExtruderAfterMerge(extruder: number, removed: number, target: number): number {
	if (extruder <= 0) return 0;
	if (extruder - 1 === removed) return target > removed ? target : target + 1;
	return remapExtruderAfterRemoval(extruder, removed);
}

/** Réglages de filament de support renumérotés à la suppression d'un slot. */
export const SUPPORT_FILAMENT_KEYS = ['support_filament', 'support_interface_filament'] as const;

/**
 * Orca Plater::on_filaments_delete : support_filament /
 * support_interface_filament (1-based, 0 = « Défaut ») égal au filament retiré
 * → effacé (« Défaut ») ; au-delà → décalé d'un cran ; sinon inchangé — y
 * compris lors d'une fusion (Orca ne les reporte pas sur le filament cible).
 * `removed` est 0-based ; renvoie null si la valeur n'est pas un entier.
 */
export function supportFilamentAfterRemoval(value: unknown, removed: number): number | null {
	const n = typeof value === 'string' ? Number(value.trim()) : value;
	if (typeof n !== 'number' || !Number.isInteger(n)) return null;
	if (n <= 0) return n;
	if (n === removed + 1) return 0;
	return n > removed + 1 ? n - 1 : n;
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
