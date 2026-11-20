// Recherche dans la liste d'objets (Orca Plater.cpp m_search_bar : « Search
// plate, object and part. ») : filtre insensible à la casse et aux accents,
// plusieurs mots = tous requis. Un objet correspond par son nom, celui d'une
// de ses pièces, ou celui de son plateau. Les ancêtres (groupes) d'un résultat
// restent affichés pour garder l'arbre lisible. Pur → testable.
import { normalizeSearch } from '../presets/search';

export interface SearchableNode {
	id: string;
	name: string;
	parentId: string | null;
}

export interface SearchablePlate {
	id: string;
	name: string;
	objectIds: readonly string[];
}

export interface ObjectSearchHit {
	id: string;
	name: string;
	plateId: string | null;
	plateName: string | null;
	/** Ce qui a correspondu (nom de l'objet, d'une pièce, ou du plateau). */
	via: 'object' | 'part' | 'plate';
	part?: string;
}

export interface ObjectSearchResult {
	/** Nœuds à afficher (résultats + leurs ancêtres). */
	shown: Set<string>;
	/** Nœuds qui correspondent eux-mêmes (surlignés). */
	matched: Set<string>;
	hits: ObjectSearchHit[];
}

function matches(text: string, terms: string[]): boolean {
	const hay = normalizeSearch(text);
	return terms.every((t) => hay.includes(t));
}

/** `null` si la recherche est vide (aucun filtre). */
export function searchObjects(
	nodes: readonly SearchableNode[],
	query: string,
	plates: readonly SearchablePlate[] = [],
	partNames: Readonly<Record<string, readonly string[]>> = {}
): ObjectSearchResult | null {
	const terms = normalizeSearch(query).split(' ').filter(Boolean);
	if (terms.length === 0) return null;
	const byId = new Map(nodes.map((n) => [n.id, n]));
	const rootOf = (id: string): string => {
		let node = byId.get(id);
		while (node?.parentId && byId.has(node.parentId)) node = byId.get(node.parentId);
		return node?.id ?? id;
	};
	const plateOf = (id: string) => {
		const root = rootOf(id);
		return plates.find((p) => p.objectIds.includes(id) || p.objectIds.includes(root)) ?? null;
	};
	const shown = new Set<string>();
	const matched = new Set<string>();
	const hits: ObjectSearchHit[] = [];
	for (const node of nodes) {
		const plate = plateOf(node.id);
		let via: ObjectSearchHit['via'] | null = null;
		let part: string | undefined;
		if (matches(node.name, terms)) via = 'object';
		else {
			part = (partNames[node.id] ?? []).find((name) => matches(name, terms));
			if (part) via = 'part';
			else if (plate && matches(plate.name, terms)) via = 'plate';
		}
		if (!via) continue;
		matched.add(node.id);
		hits.push({
			id: node.id,
			name: node.name,
			plateId: plate?.id ?? null,
			plateName: plate?.name ?? null,
			via,
			...(part ? { part } : {})
		});
		for (let cur: SearchableNode | undefined = node; cur;) {
			shown.add(cur.id);
			cur = cur.parentId ? byId.get(cur.parentId) : undefined;
		}
	}
	return { shown, matched, hits };
}
