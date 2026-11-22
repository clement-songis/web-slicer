// Séquences de filaments d'un plateau (audit 45 MM-b) — Orca
// PlateSettingsDialog.cpp (DragCanvas, OtherLayersSeqPanel, LayerSeqInfo) et
// PartPlate::{get,set}_{first_layer,other_layers}_print_sequence :
// - `first_layer_print_sequence` = ordre des filaments (1-based) de la
//   1ʳᵉ couche, absent = « Auto » ;
// - `other_layers_print_sequence` = blocs [début, fin, ordre…] (fin =
//   MAX_LAYER_VALUE = « Fin »), `other_layers_print_sequence_nums` = nombre de
//   blocs (ParameterUtils.cpp get_other_layers_print_sequence).
// Valeurs écrites au format texte d'Orca (« 2,1,3 »), comme les réglages de
// plateau relus d'un 3MF. Pur → testable.

/** Couche de début minimale d'une plage (MIN_LAYER_VALUE d'Orca). */
export const MIN_LAYER_VALUE = 2;
/** « Fin » (MAX_LAYER_VALUE = INT_MAX − 1). */
export const MAX_LAYER_VALUE = 2147483646;

export interface LayerSeqRange {
	begin: number;
	end: number;
	/** Ordre des filaments, numéros 1-based. */
	order: number[];
}

function ints(raw: unknown): number[] {
	const list = Array.isArray(raw) ? raw : typeof raw === 'string' ? raw.split(/[,;\s]+/) : [raw];
	return list
		.map((v) => (typeof v === 'number' ? v : Number(String(v ?? '').trim())))
		.filter((v) => Number.isInteger(v));
}

/** Ordre par défaut d'un DragCanvas : 1..n. */
export function defaultOrder(n: number): number[] {
	return Array.from({ length: Math.max(0, n) }, (_, i) => i + 1);
}

/**
 * Ordre complet pour n filaments (PartPlate::update_first_layer_print_sequence) :
 * numéros valides et uniques conservés dans leur ordre, filaments manquants
 * ajoutés à la fin.
 */
export function normalizeOrder(order: readonly number[], n: number): number[] {
	const seen = new Set<number>();
	const out: number[] = [];
	for (const v of order) {
		if (v >= 1 && v <= n && !seen.has(v)) {
			seen.add(v);
			out.push(v);
		}
	}
	for (let i = 1; i <= n; i++) if (!seen.has(i)) out.push(i);
	return out;
}

/** Ordre de la 1ʳᵉ couche lu d'un réglage de plateau (absent / « 0 » → null = Auto). */
export function parseFirstLayerSequence(raw: unknown, n: number): number[] | null {
	const values = ints(raw);
	if (values.length === 0 || (values.length === 1 && values[0] === 0)) return null;
	return normalizeOrder(values, n);
}

/** Plages des autres couches lues des deux réglages (absents → []). */
export function parseOtherLayersSequence(
	rawSequence: unknown,
	rawNums: unknown,
	n: number
): LayerSeqRange[] {
	const seq = ints(rawSequence);
	const nums = ints(rawNums)[0] ?? 0;
	if (nums <= 0 || seq.length === 0 || seq.length % nums !== 0) return [];
	const size = seq.length / nums;
	if (size < 2) return [];
	const out: LayerSeqRange[] = [];
	for (let k = 0; k < nums; k++) {
		const block = seq.slice(k * size, (k + 1) * size);
		out.push({
			begin: Math.max(MIN_LAYER_VALUE, block[0]),
			end: Math.min(MAX_LAYER_VALUE, block[1]),
			order: normalizeOrder(block.slice(2), n)
		});
	}
	return out;
}

/** Texte Orca d'un ordre (« 2,1,3 »). */
export function formatOrder(order: readonly number[]): string {
	return order.join(',');
}

/** Réglages `other_layers_print_sequence` / `_nums` d'une liste de plages. */
export function formatOtherLayersSequence(ranges: readonly LayerSeqRange[]): {
	sequence: string;
	nums: string;
} {
	const seq: number[] = [];
	for (const r of ranges) seq.push(r.begin, r.end, ...r.order);
	return { sequence: seq.join(','), nums: String(ranges.length) };
}

/** Plage valide : début ≥ 2, fin ≥ début (LayerNumberTextInput). */
export function rangeValid(r: LayerSeqRange): boolean {
	return (
		Number.isInteger(r.begin) &&
		Number.isInteger(r.end) &&
		r.begin >= MIN_LAYER_VALUE &&
		r.end >= r.begin
	);
}

/** Tri d'Orca (LayerSeqInfo::operator<) : par début puis fin. */
export function sortRanges(ranges: readonly LayerSeqRange[]): LayerSeqRange[] {
	return [...ranges].sort((a, b) => a.begin - b.begin || a.end - b.end);
}

/**
 * Nouvelle plage (OtherLayersSeqPanel::append_layer) : la première couvre
 * « couche 2 → Fin », les suivantes sont à saisir (début = fin = 2 ici, pour
 * rester une plage valide), ordre par défaut.
 */
export function newRange(existing: number, n: number): LayerSeqRange {
	return existing === 0
		? { begin: MIN_LAYER_VALUE, end: MAX_LAYER_VALUE, order: defaultOrder(n) }
		: { begin: MIN_LAYER_VALUE, end: MIN_LAYER_VALUE, order: defaultOrder(n) };
}

/** Déplace l'élément d'indice `from` à l'indice `to` (glisser-déposer). */
export function moveItem<T>(list: readonly T[], from: number, to: number): T[] {
	if (from === to || from < 0 || to < 0 || from >= list.length || to >= list.length)
		return [...list];
	const out = [...list];
	const [item] = out.splice(from, 1);
	out.splice(to, 0, item);
	return out;
}
