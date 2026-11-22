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

/** Champ de couche vide (LayerNumberTextInput : m_layer_number = −1). */
export const EMPTY_LAYER = -1;

/** Numéro de couche saisi (≥ 2) ; vide / invalide = EMPTY_LAYER. */
export function layerSet(v: number): boolean {
	return Number.isInteger(v) && v >= MIN_LAYER_VALUE;
}

/** Plage valide : début ≥ 2, fin ≥ début (LayerNumberTextInput). */
export function rangeValid(r: LayerSeqRange): boolean {
	return layerSet(r.begin) && layerSet(r.end) && r.end >= r.begin;
}

/**
 * Plages remplies (début saisi) — audit 49 PLATE49-a. Les lignes ajoutées
 * par « Ajouter » puis jamais remplies (début vide, fin « Fin », ordre par
 * défaut : `newRange` d'`append_layer`) sont des lignes d'attente : elles ne
 * sont ni validées ni sérialisées à l'application, comme les plages vides
 * d'Orca n'invalident pas les plages saisies. Une plage à début saisi mais
 * fin vide/inférieure reste invalide (message de saisie).
 */
export function filledLayerRanges<T extends LayerSeqRange>(ranges: readonly T[]): T[] {
	return ranges.filter((r) => layerSet(r.begin));
}

/**
 * Comparaison d'Orca (LayerSeqInfo::operator<) : par début puis fin, les
 * champs vides (< MIN_LAYER_VALUE) en dernier.
 */
function layerSeqLess(a: LayerSeqRange, b: LayerSeqRange): boolean {
	if (a.begin < MIN_LAYER_VALUE) return false;
	if (b.begin < MIN_LAYER_VALUE) return true;
	if (a.begin === b.begin) {
		if (a.end < MIN_LAYER_VALUE) return false;
		if (b.end < MIN_LAYER_VALUE) return true;
		return a.end < b.end;
	}
	return a.begin < b.begin;
}

/**
 * Tri d'Orca (EVT_NEED_RESORT_LAYERS : std::sort si la liste n'est pas déjà
 * triée), stable ; les éléments gardent leur identité (lignes à clé stable).
 */
export function sortRanges<T extends LayerSeqRange>(ranges: readonly T[]): T[] {
	return [...ranges].sort((a, b) => (layerSeqLess(a, b) ? -1 : layerSeqLess(b, a) ? 1 : 0));
}

function parseLayer(raw: string): number | null {
	const s = raw.trim();
	if (s === '') return null;
	const v = Math.round(Number(s));
	if (!Number.isFinite(v)) return null;
	return Math.min(MAX_LAYER_VALUE, Math.max(MIN_LAYER_VALUE, v));
}

/**
 * Validation du début saisi (LayerNumberTextInput::validate_input_value,
 * Type::Begin) : vide → champ vide ; valeur bornée à [2, Fin] ; une fin
 * renseignée inférieure est repoussée au début.
 */
export function commitBegin<T extends LayerSeqRange>(r: T, raw: string): T {
	const v = parseLayer(raw);
	if (v === null) return { ...r, begin: EMPTY_LAYER };
	return { ...r, begin: v, end: layerSet(r.end) && v > r.end ? v : r.end };
}

/**
 * Validation de la fin saisie (Type::End) : vide → champ vide ; inférieure
 * au début → refusée (null : valeur précédente conservée) si la fin était
 * renseignée, sinon fin = début.
 */
export function commitEnd<T extends LayerSeqRange>(r: T, raw: string): T | null {
	const v = parseLayer(raw);
	if (v === null) return { ...r, end: EMPTY_LAYER };
	if (layerSet(r.begin) && r.begin > v) return layerSet(r.end) ? null : { ...r, end: r.begin };
	return { ...r, end: v };
}

/**
 * Nouvelle plage (OtherLayersSeqPanel::append_layer) : la première couvre
 * « couche 2 → Fin », les suivantes ont un début vide (à saisir) et une fin
 * « Fin », ordre par défaut — vide, elle est triée après les autres.
 */
export function newRange(existing: number, n: number): LayerSeqRange {
	return existing === 0
		? { begin: MIN_LAYER_VALUE, end: MAX_LAYER_VALUE, order: defaultOrder(n) }
		: { begin: EMPTY_LAYER, end: MAX_LAYER_VALUE, order: defaultOrder(n) };
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
