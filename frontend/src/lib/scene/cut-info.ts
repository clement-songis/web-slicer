// Informations de coupe (lot 37, Orca `ModelObject::cut_id` / CutObjectBase) :
// les pièces issues d'une même coupe partagent un `cutId` `{ id, checkSum,
// connectorsCnt }` (recopié de `parts[].cut_id` de POST /cut). Orca les traite
// comme liées : sélection groupée (ObjectList::fix_cut_selection, même `id`),
// échelle uniforme seulement, « Invalider les informations de découpe »
// (invalidate_cut_info_for_object : objets de `cutId` strictement égal). Pur.

/** `cutId` d'un objet de scène (forme du serveur). */
export interface CutObjectId {
	id: number;
	checkSum: number;
	connectorsCnt: number;
}

/** Lecture tolérante d'un `cutId` (absent, nul ou d'id 0 → `null`, comme le serveur). */
export function parseCutId(raw: unknown): CutObjectId | null {
	if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return null;
	const c = raw as Partial<CutObjectId>;
	const int = (v: unknown) => (typeof v === 'number' && Number.isFinite(v) ? Math.trunc(v) : 0);
	const id = int(c.id);
	if (id <= 0) return null;
	return { id, checkSum: int(c.checkSum), connectorsCnt: int(c.connectorsCnt) };
}

/** CutObjectBase::has_same_id. */
export function sameCutId(a: CutObjectId | null | undefined, b: CutObjectId | null | undefined) {
	return !!a && !!b && a.id === b.id;
}

/** CutObjectBase::is_equal (id, somme de contrôle et connecteurs). */
export function equalCutId(a: CutObjectId | null | undefined, b: CutObjectId | null | undefined) {
	return sameCutId(a, b) && a!.checkSum === b!.checkSum && a!.connectorsCnt === b!.connectorsCnt;
}

interface CutNode {
	id: string;
	cutId?: CutObjectId | null;
}

/** Pièces liées à `id` (lui compris) : même `cutId.id` ; `[id]` si non coupé. */
export function cutLinkedIds(nodes: readonly CutNode[], id: string): string[] {
	const cut = nodes.find((n) => n.id === id)?.cutId;
	if (!cut) return [id];
	return nodes.filter((n) => sameCutId(n.cutId, cut)).map((n) => n.id);
}

/**
 * Sélection complétée des pièces liées (fix_cut_selection) : chaque objet coupé
 * entraîne les autres pièces de sa coupe ; l'ordre de la sélection est gardé.
 */
export function withCutPartners(nodes: readonly CutNode[], selection: Iterable<string>): string[] {
	const out: string[] = [];
	for (const id of selection)
		for (const linked of cutLinkedIds(nodes, id)) if (!out.includes(linked)) out.push(linked);
	return out;
}

/** Objets dont « Invalider les informations de découpe » retire le `cutId`. */
export function invalidatedIds(nodes: readonly CutNode[], id: string): string[] {
	const cut = nodes.find((n) => n.id === id)?.cutId;
	if (!cut) return [];
	return nodes.filter((n) => equalCutId(n.cutId, cut)).map((n) => n.id);
}

/**
 * Échelle d'une pièce coupée : uniforme seulement (Orca force l'échelle
 * uniforme d'un objet coupé). L'axe le plus modifié (rapport à l'échelle
 * précédente) donne le facteur, appliqué aux trois axes.
 */
export function uniformCutScale(
	previous: readonly [number, number, number],
	next: readonly [number, number, number]
): [number, number, number] {
	let ratio = 1;
	let best = 0;
	for (let a = 0; a < 3; a++) {
		const base = previous[a] || 1;
		const r = next[a] / base;
		const delta = Math.abs(r - 1);
		if (delta > best + 1e-12) {
			best = delta;
			ratio = r;
		}
	}
	const s = previous.map((p) => Math.round((p || 1) * ratio * 1e6) / 1e6) as [
		number,
		number,
		number
	];
	return s;
}

/**
 * synchronize_model_after_cut : quand la coupe reprend un id existant (objet
 * déjà coupé), les autres objets de même `cutId.id` reçoivent le nouveau
 * `cutId`. Renvoie les ids à mettre à jour (hors pièces neuves).
 */
export function syncAfterCut(
	nodes: readonly CutNode[],
	cut: CutObjectId | null,
	exclude: readonly string[]
): string[] {
	if (!cut) return [];
	return nodes.filter((n) => !exclude.includes(n.id) && sameCutId(n.cutId, cut)).map((n) => n.id);
}

/** Libellé du type de connecteur (Orca CutConnectorType). */
export const CONNECTOR_TYPE_LABELS: Record<'plug' | 'dowel' | 'snap', string> = {
	plug: 'Tenon',
	dowel: 'Goujon',
	snap: 'Clip'
};
