// Instances d'un objet (audit 43 F-a, parité Orca 2.4.1) : Plater::increase_instances
// (Plater.cpp:14656) ajoute des instances du même ModelObject décalées en
// escalier depuis la dernière : k × 5 % du plus grand côté du plateau en X et Y
// (GLCanvas3D::get_size_proportional_to_max_bed_size(0.05)) ;
// Plater::set_number_of_copies (Plater.cpp:14747) demande « Number of copies: »
// (0–1000, défaut = nombre d'instances actuel) puis ajoute/retire la
// différence (0 → l'objet est supprimé, decrease_instances → remove).
//
// Côté web, une instance est un objet de scène partageant le modèle serveur et
// un identifiant de groupe (`instanceGroup`, persisté dans objects[]) : les
// réglages, l'extrudeur, le nom, les volumes et plages de hauteur d'un membre
// sont recopiés sur les autres (comme les données d'un ModelObject partagées
// par ses ModelInstance) ; seule la transformation est propre à l'instance.
// Fonctions pures → testables sans Svelte.
import {
	eulerDegFromQuaternion,
	quaternionFromEulerDeg,
	rotateVectorByEulerDeg,
	type Transform
} from './transform';

/** Facteur d'Orca (get_size_proportional_to_max_bed_size(0.05)). */
export const INSTANCE_OFFSET_FACTOR = 0.05;
/** Bornes du dialogue d'Orca (GetNumberFromUser(…, 0, 1000)). */
export const MAX_INSTANCES = 1000;

/** Pas du décalage en escalier pour un plateau de `width` × `depth` mm. */
export function instanceStep(bed: { width: number; depth: number }): number {
	const size = Math.max(bed.width, bed.depth);
	return INSTANCE_OFFSET_FACTOR * (Number.isFinite(size) && size > 0 ? size : 256);
}

/** Positions des `num` nouvelles instances depuis la dernière (k × pas en X et Y). */
export function instancePositions(
	last: readonly [number, number, number],
	num: number,
	step: number
): [number, number, number][] {
	const out: [number, number, number][] = [];
	for (let k = 1; k <= num; k++) out.push([last[0] + k * step, last[1] + k * step, last[2]]);
	return out;
}

/**
 * Saisie du dialogue : entier de 0 à 1000 (virgule ou point acceptés, comme
 * les champs numériques de l'éditeur) ; sinon `null`.
 */
export function parseInstanceCount(text: string): number | null {
	const t = text.trim().replace(',', '.');
	if (!/^\d+(\.0*)?$/.test(t)) return null;
	const n = Number(t);
	return Number.isInteger(n) && n >= 0 && n <= MAX_INSTANCES ? n : null;
}

/** Ajouts / retraits pour passer de `current` à `target` instances. */
export function instanceCountPlan(
	current: number,
	target: number
): { add: number; remove: number; deleteObject: boolean } {
	const diff = target - current;
	if (diff >= 0) return { add: diff, remove: 0, deleteObject: false };
	// decrease_instances : moins d'instances que demandé à retirer → remove(obj).
	if (target <= 0) return { add: 0, remove: current, deleteObject: true };
	return { add: 0, remove: -diff, deleteObject: false };
}

/** Nouvel identifiant de groupe d'instances (opaque, stable à la sauvegarde). */
export function newInstanceGroupId(): string {
	const rnd =
		typeof crypto !== 'undefined' && 'randomUUID' in crypto
			? crypto.randomUUID()
			: Math.random().toString(36).slice(2) + Date.now().toString(36);
	return `ig-${rnd}`;
}

/** Groupes d'instances (≥ 2 membres) dans l'ordre des objets. */
/**
 * Phase 46 (audit 44 FIX1-a) — liste d'objets : les instances d'un objet sont
 * regroupées sous le premier membre (Orca ObjectList : nœud objet + sous-nœud
 * « Instances », ObjectList::increase_object_instances → add_instance), au lieu
 * d'apparaître comme objets frères intercalés. Ordre : position du premier
 * membre ; `instances` = les autres membres (2ᵉ, 3ᵉ…), dans l'ordre de la liste.
 */
export function instanceListLayout<T extends { id: string; instanceGroup?: string }>(
	roots: readonly T[]
): { node: T; instances: T[] }[] {
	const groups = instanceGroups(roots);
	const out: { node: T; instances: T[] }[] = [];
	const byLeader = new Map<string, { node: T; instances: T[] }>();
	for (const n of roots) {
		const ids = n.instanceGroup ? groups.get(n.instanceGroup) : undefined;
		if (!ids) {
			out.push({ node: n, instances: [] });
			continue;
		}
		const leader = byLeader.get(n.instanceGroup!);
		if (leader) leader.instances.push(n);
		else {
			const entry = { node: n, instances: [] as T[] };
			byLeader.set(n.instanceGroup!, entry);
			out.push(entry);
		}
	}
	return out;
}

export function instanceGroups(
	nodes: readonly { id: string; instanceGroup?: string }[]
): Map<string, string[]> {
	const groups = new Map<string, string[]>();
	for (const n of nodes) {
		if (!n.instanceGroup) continue;
		const list = groups.get(n.instanceGroup) ?? [];
		list.push(n.id);
		groups.set(n.instanceGroup, list);
	}
	for (const [g, ids] of groups) if (ids.length < 2) groups.delete(g);
	return groups;
}

/**
 * Propagation des données partagées : pour chaque groupe, le membre dont la
 * signature diffère de la dernière connue (`last`) devient la source et ses
 * données sont recopiées sur les autres. Aucun changement ou groupe déjà
 * homogène → rien. Met `last` à jour (mutation voulue : mémoire de l'effet).
 */
export function planInstanceSync(
	groups: Map<string, string[]>,
	signature: (id: string) => string,
	last: Map<string, string>
): { source: string; targets: string[] }[] {
	const plan: { source: string; targets: string[] }[] = [];
	for (const [group, ids] of groups) {
		const sigs = ids.map(signature);
		const known = last.get(group);
		const changed = ids.findIndex((_, i) => sigs[i] !== known);
		if (sigs.every((s) => s === sigs[0])) {
			last.set(group, sigs[0]);
			continue;
		}
		// Groupe hétérogène : le premier membre modifié depuis la dernière
		// synchronisation fait foi (à défaut, le premier du groupe).
		const sourceIndex = known === undefined ? 0 : Math.max(0, changed);
		const source = ids[sourceIndex];
		plan.push({ source, targets: ids.filter((id) => id !== source) });
		last.set(group, sigs[sourceIndex]);
	}
	for (const group of [...last.keys()]) if (!groups.has(group)) last.delete(group);
	return plan;
}

// — Phase 44b : données lourdes partagées (peinture, oreilles de bordure,
// profil de hauteur de couche), comme `ModelVolume` (peinture) et
// `ModelObject::brim_points` / `layer_height_profile` d'Orca. Les signatures
// sont coûteuses à comparer d'un membre à l'autre (oreilles en coordonnées
// plateau, propres à chaque instance) : on suit donc la dernière signature
// connue **par membre** et le membre modifié fait foi.

/**
 * Membres modifiés depuis la dernière synchronisation : pour chaque groupe, le
 * premier membre dont la signature diffère de celle mémorisée devient la
 * source. Un membre encore inconnu (instance qui vient d'être créée, déjà
 * copiée de sa source) est seulement mémorisé. L'appelant met la mémoire à
 * jour après avoir recopié (`rememberMembers`).
 */
export function planMemberSync(
	groups: Map<string, string[]>,
	signature: (id: string) => string,
	memory: Map<string, string>
): { source: string; targets: string[] }[] {
	const plan: { source: string; targets: string[] }[] = [];
	const members = new Set<string>();
	for (const ids of groups.values()) {
		let source: string | null = null;
		for (const id of ids) {
			members.add(id);
			const sig = signature(id);
			const known = memory.get(id);
			if (known === undefined) memory.set(id, sig);
			else if (known !== sig && source === null) source = id;
		}
		if (source !== null) plan.push({ source, targets: ids.filter((id) => id !== source) });
	}
	for (const id of [...memory.keys()]) if (!members.has(id)) memory.delete(id);
	return plan;
}

/** Mémorise la signature actuelle des membres (après recopie). */
export function rememberMembers(
	ids: Iterable<string>,
	signature: (id: string) => string,
	memory: Map<string, string>
): void {
	for (const id of ids) memory.set(id, signature(id));
}

/**
 * Oreilles de bordure (points plateau, mm) d'une instance reportées sur une
 * autre : point → repère de l'objet source (inverse de sa transformation) →
 * plateau via la transformation de la cible. Orca garde les points dans le
 * repère de l'objet (`ModelObject::brim_points`), partagés par les instances.
 * Les points sont au niveau du plateau (z = 0).
 */
export function mapPlatePoints(
	points: readonly (readonly [number, number])[],
	from: Transform,
	to: Transform
): [number, number][] {
	const [qx, qy, qz, qw] = quaternionFromEulerDeg(from.rotation);
	const inverse = eulerDegFromQuaternion([-qx, -qy, -qz, qw]);
	const round = (v: number) => Math.round(v * 1e6) / 1e6 || 0;
	return points.map(([x, y]) => {
		const d = rotateVectorByEulerDeg(inverse, [
			x - from.position[0],
			y - from.position[1],
			0 - from.position[2]
		]);
		const local: [number, number, number] = [0, 1, 2].map((a) => d[a] / (from.scale[a] || 1)) as [
			number,
			number,
			number
		];
		const w = rotateVectorByEulerDeg(to.rotation, [
			local[0] * to.scale[0],
			local[1] * to.scale[1],
			local[2] * to.scale[2]
		]);
		return [round(w[0] + to.position[0]), round(w[1] + to.position[1])];
	});
}
