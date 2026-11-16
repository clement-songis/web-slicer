// Ordre d'impression séquentielle (« objet par objet ») : comme Orca, l'ordre
// des objets dans la liste d'objets est l'ordre du modèle, donc l'ordre
// d'impression (`sort_object_instances_by_model_order`, GCode.cpp). On le
// change en glissant un objet sur un autre (ObjectList::OnDrop : l'objet
// glissé prend la place de la cible, les autres décalés d'un rang).

/** Déplace l'élément `id` à la place de `targetId` (vers le haut : avant la
 *  cible ; vers le bas : après). Renvoie la liste inchangée si l'un des deux
 *  est absent ou s'ils sont identiques. */
export function moveToTarget<T>(
	list: readonly T[],
	id: string,
	targetId: string,
	key: (item: T) => string
): T[] {
	const from = list.findIndex((item) => key(item) === id);
	const to = list.findIndex((item) => key(item) === targetId);
	if (from < 0 || to < 0 || from === to) return [...list];
	const out = [...list];
	const [moved] = out.splice(from, 1);
	out.splice(to, 0, moved);
	return out;
}

/** Déplace `id` d'un rang parmi les éléments retenus par `among` (clavier :
 *  Alt+↑ / Alt+↓). Renvoie l'id de l'élément échangé, ou null. */
export function neighbourOf<T>(
	list: readonly T[],
	id: string,
	delta: -1 | 1,
	key: (item: T) => string,
	among: (item: T) => boolean
): string | null {
	const eligible = list.filter(among).map(key);
	const at = eligible.indexOf(id);
	if (at < 0) return null;
	return eligible[at + delta] ?? null;
}

/** Concatène `added` à `existing` puis remet dans l'ordre de `rank` les
 *  éléments classés (objets restaurés d'une sauvegarde, rang = ordre du
 *  document), sans bouger les autres : les maillages arrivent dans le désordre
 *  (un chargement par modèle), l'ordre d'impression doit rester celui sauvegardé. */
export function mergeInRank<T>(
	existing: readonly T[],
	added: readonly T[],
	rank: (item: T) => number | undefined
): T[] {
	const all = [...existing, ...added];
	const slots: number[] = [];
	const ranked: T[] = [];
	all.forEach((item, i) => {
		if (rank(item) !== undefined) {
			slots.push(i);
			ranked.push(item);
		}
	});
	ranked.sort((a, b) => rank(a)! - rank(b)!);
	slots.forEach((slot, i) => (all[slot] = ranked[i]));
	return all;
}

/** Séquence effective du plateau : surcharge du plateau, sinon le traitement. */
export function isByObject(plateValue: unknown, processValue: unknown): boolean {
	const v =
		plateValue !== undefined && plateValue !== null && plateValue !== ''
			? plateValue
			: processValue;
	return v === 'by object';
}

/** Champs du mode séquentiel de `POST /arrange` (ArrangeRequest) : ArrangeParams
 *  d'Orca en objet par objet (init_arrange_params). `nozzle_height` est accepté
 *  mais sans effet côté moteur, comme le CLI Orca. */
export interface SequentialArrangeFields {
	is_seq_print: true;
	extruder_clearance_radius: number;
	extruder_clearance_height_to_rod: number;
	extruder_clearance_height_to_lid: number;
	nozzle_height: number;
}

const num = (v: unknown, fallback: number): number => {
	const n = Number(Array.isArray(v) ? v[0] : v);
	return Number.isFinite(n) ? n : fallback;
};

/** Valeurs machine prioritaires (onglet Imprimante d'Orca), puis traitement,
 *  puis défauts Orca (PrintConfig.cpp : 40 / 40 / 120 / 2,5 mm). */
export function sequentialArrangeFields(
	machine: Record<string, unknown>,
	process: Record<string, unknown>
): SequentialArrangeFields {
	const pick = (key: string, fallback: number) => num(machine[key] ?? process[key], fallback);
	return {
		is_seq_print: true,
		extruder_clearance_radius: pick('extruder_clearance_radius', 40),
		extruder_clearance_height_to_rod: pick('extruder_clearance_height_to_rod', 40),
		extruder_clearance_height_to_lid: pick('extruder_clearance_height_to_lid', 120),
		nozzle_height: pick('nozzle_height', 2.5)
	};
}

/** Ordre d'impression renvoyé par un arrangement séquentiel (`print_order`
 *  par placement, Orca `arrange_order`) ; null si absent ou incomplet (l'ordre
 *  de la liste ne change pas). */
export function orderFromPlacements(
	placements: readonly { id: string; print_order?: number | null }[]
): string[] | null {
	if (placements.length === 0) return null;
	if (!placements.every((p) => typeof p.print_order === 'number' && Number.isFinite(p.print_order)))
		return null;
	return [...placements].sort((a, b) => a.print_order! - b.print_order!).map((p) => p.id);
}

/** Réordonne les éléments listés dans `order` en gardant les emplacements
 *  qu'ils occupent (les autres ne bougent pas). */
export function permuteInSlots<T>(
	list: readonly T[],
	order: readonly string[],
	key: (item: T) => string
): T[] {
	const rank = new Map(order.map((id, i) => [id, i]));
	return mergeInRank([], list, (item) => rank.get(key(item)));
}
