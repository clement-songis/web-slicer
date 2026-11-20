// Arbre d'objets de la scène (T053) : modèle pour la liste d'objets Orca —
// objets/pièces/groupes, sélection, duplication, verrouillage/masquage,
// extrudeur par objet/pièce et surcharges de réglages par objet (FR-015).
// Réactif : le tableau `nodes` est en `$state` (proxy profond) pour que la
// couche Svelte se re-rende à chaque mutation. Runes → testé sous vitest
// (`.svelte.vitest.ts`), pas sous bun.

import { moveToTarget, permuteInSlots } from './print-order';
import type { CutObjectId } from './cut-info';

/** Type de nœud : objet racine, pièce d'un objet, ou groupe d'objets. */
export type NodeKind = 'object' | 'part' | 'group';

/** Nœud de l'arbre d'objets. */
export interface SceneNode {
	id: string;
	name: string;
	kind: NodeKind;
	/** Parent (groupe ou objet parent), ou `null` à la racine. */
	parentId: string | null;
	/** Verrou : non déplaçable/éditable dans la scène (US4). */
	locked: boolean;
	/** Masqué : non rendu (l'état effectif hérite des ancêtres). */
	hidden: boolean;
	/** Imprimable : exclu du tranchage et affiché grisé si false (V). */
	printable: boolean;
	/** Extrudeur assigné (0 = hérité du réglage global). */
	extruder: number;
	/** Surcharges de réglages par objet (FR-015), par clé de paramètre. */
	settings: Record<string, unknown>;
	/**
	 * Nom du volume porté par le maillage propre de l'objet (Orca : 1er
	 * `ModelVolume`, `<nom>_1` après un split) ; absent = nom de l'objet.
	 */
	meshName?: string;
	/** Phase 28 — objet texte / SVG en relief ré-éditable (objects[].emboss). */
	emboss?: Record<string, unknown>;
	/** Lot 37 — pièce issue d'une coupe (Orca `ModelObject::cut_id`). */
	cutId?: CutObjectId;
}

function cloneNode(n: SceneNode): SceneNode {
	return { ...n, settings: { ...n.settings } };
}

/** Arbre d'objets avec opérations de la liste d'objets. Ordre d'insertion préservé. */
export class ObjectTree {
	private nodes = $state<SceneNode[]>([]);
	private counter = 0;

	/** Génère un identifiant déterministe (utile pour les tests). */
	private nextId(): string {
		this.counter += 1;
		return `n${this.counter}`;
	}

	/** Ajoute un objet (par défaut à la racine). */
	add(name: string, opts: Partial<Omit<SceneNode, 'id' | 'name'>> = {}): SceneNode {
		const node: SceneNode = {
			id: this.nextId(),
			name,
			kind: opts.kind ?? 'object',
			parentId: opts.parentId ?? null,
			locked: opts.locked ?? false,
			hidden: opts.hidden ?? false,
			printable: opts.printable ?? true,
			extruder: opts.extruder ?? 0,
			settings: { ...(opts.settings ?? {}) },
			...(opts.meshName ? { meshName: opts.meshName } : {}),
			...(opts.emboss ? { emboss: opts.emboss } : {})
		};
		this.nodes.push(node);
		return node;
	}

	/** Liste des nœuds dans l'ordre d'affichage. */
	list(): readonly SceneNode[] {
		return this.nodes;
	}

	get(id: string): SceneNode | undefined {
		return this.nodes.find((n) => n.id === id);
	}

	/** Enfants directs d'un nœud (ou racines si `parentId` est `null`). */
	children(parentId: string | null): SceneNode[] {
		return this.nodes.filter((n) => n.parentId === parentId);
	}

	/** Nœuds racines. */
	roots(): SceneNode[] {
		return this.children(null);
	}

	private descendants(id: string): SceneNode[] {
		const out: SceneNode[] = [];
		const stack = this.children(id);
		while (stack.length) {
			const n = stack.pop()!;
			out.push(n);
			stack.push(...this.children(n.id));
		}
		return out;
	}

	/**
	 * Duplique un nœud (et son sous-arbre s'il s'agit d'un groupe/objet à pièces),
	 * avec de nouveaux identifiants. Le clone racine reçoit le suffixe « (copie) ».
	 */
	duplicate(id: string): SceneNode | undefined {
		const src = this.get(id);
		if (!src) return undefined;
		const subtree = [src, ...this.descendants(id)];
		// Table locale ancien→nouvel id (non réactive) : usage transitoire.
		// eslint-disable-next-line svelte/prefer-svelte-reactivity
		const idMap = new Map<string, string>();
		for (const n of subtree) idMap.set(n.id, this.nextId());
		let root: SceneNode | undefined;
		for (const n of subtree) {
			const clone = cloneNode(n);
			clone.id = idMap.get(n.id)!;
			clone.parentId = n.parentId === null ? null : (idMap.get(n.parentId) ?? n.parentId);
			if (n.id === src.id) {
				clone.name = `${n.name} (copie)`;
				root = clone;
			}
			this.nodes.push(clone);
		}
		return root;
	}

	/**
	 * Regroupe des nœuds racines sous un nouveau groupe (les autres ids sont
	 * ignorés). Renvoie le groupe créé, ou `undefined` si moins de deux nœuds.
	 */
	group(ids: string[], name = 'Groupe'): SceneNode | undefined {
		const members = ids.map((id) => this.get(id)).filter((n): n is SceneNode => n !== undefined);
		if (members.length < 2) return undefined;
		const parentId = members[0].parentId;
		const grp = this.add(name, { kind: 'group', parentId });
		for (const m of members) m.parentId = grp.id;
		return grp;
	}

	/**
	 * Déplace un nœud juste après `afterId` dans l'ordre d'affichage (même
	 * parent conservé). Sert à insérer les morceaux d'un split à la place de la
	 * pièce source (Orca `ModelVolume::split` : `volumes.insert(ivolume + 1)`).
	 */
	moveAfter(id: string, afterId: string): void {
		const node = this.get(id);
		if (!node || id === afterId || !this.get(afterId)) return;
		const rest = this.nodes.filter((n) => n.id !== id);
		const at = rest.findIndex((n) => n.id === afterId);
		this.nodes = [...rest.slice(0, at + 1), node, ...rest.slice(at + 1)];
	}

	/**
	 * Ordre des objets (= ordre d'impression séquentielle, comme le modèle
	 * d'Orca) : `id` prend la place de `targetId` (glisser-déposer de la liste
	 * d'objets, ObjectList::OnDrop). Les deux nœuds doivent partager le parent.
	 */
	moveTo(id: string, targetId: string): boolean {
		const node = this.get(id);
		const target = this.get(targetId);
		if (!node || !target || id === targetId || node.parentId !== target.parentId) return false;
		this.nodes = moveToTarget(this.nodes, id, targetId, (n) => n.id);
		return true;
	}

	/** Applique un ordre (ids) aux nœuds listés, sans déplacer les autres
	 *  (ordre d'impression renvoyé par un arrangement séquentiel). */
	setOrder(ids: readonly string[]): void {
		this.nodes = permuteInSlots(this.nodes, ids, (n) => n.id);
	}

	/** Dissout un groupe : ses enfants remontent au parent du groupe. */
	ungroup(groupId: string): boolean {
		const grp = this.get(groupId);
		if (!grp || grp.kind !== 'group') return false;
		for (const child of this.children(groupId)) child.parentId = grp.parentId;
		this.nodes = this.nodes.filter((n) => n.id !== groupId);
		return true;
	}

	/** Supprime un nœud et tout son sous-arbre. */
	remove(id: string): boolean {
		if (!this.get(id)) return false;
		// Ensemble local des ids à supprimer (non réactif) : usage transitoire.
		// eslint-disable-next-line svelte/prefer-svelte-reactivity
		const doomed = new Set([id, ...this.descendants(id).map((n) => n.id)]);
		this.nodes = this.nodes.filter((n) => !doomed.has(n.id));
		return true;
	}

	rename(id: string, name: string): void {
		const n = this.get(id);
		const trimmed = name.trim();
		if (n && trimmed) n.name = trimmed;
	}

	/** Copie profonde des nœuds (historique undo/redo). */
	snapshot(): SceneNode[] {
		return this.nodes.map(cloneNode);
	}

	/** Restaure des nœuds issus de `snapshot()` (le compteur d'ids ne recule pas). */
	restore(nodes: readonly SceneNode[]): void {
		this.nodes = nodes.map(cloneNode);
		for (const n of nodes) {
			const m = /^n(\d+)$/.exec(n.id);
			if (m) this.counter = Math.max(this.counter, Number(m[1]));
		}
	}

	setLocked(id: string, locked: boolean): void {
		const n = this.get(id);
		if (n) n.locked = locked;
	}

	setHidden(id: string, hidden: boolean): void {
		const n = this.get(id);
		if (n) n.hidden = hidden;
	}

	setPrintable(id: string, printable: boolean): void {
		const n = this.get(id);
		if (n) n.printable = printable;
	}

	isPrintable(id: string): boolean {
		const n = this.get(id);
		return !!n && n.printable && this.ancestors(id).every((a) => a.printable);
	}

	setExtruder(id: string, extruder: number): void {
		const n = this.get(id);
		if (n) n.extruder = Math.max(0, Math.trunc(extruder));
	}

	/** Définit (ou efface si `undefined`) une surcharge de réglage par objet. */
	setSetting(id: string, key: string, value: unknown): void {
		const n = this.get(id);
		if (!n) return;
		if (value === undefined) delete n.settings[key];
		else n.settings[key] = value;
	}

	private ancestors(id: string): SceneNode[] {
		const out: SceneNode[] = [];
		let cur = this.get(id);
		while (cur && cur.parentId !== null) {
			const parent = this.get(cur.parentId);
			if (!parent) break;
			out.push(parent);
			cur = parent;
		}
		return out;
	}

	/** Masquage effectif : le nœud ou un de ses ancêtres est masqué. */
	isHidden(id: string): boolean {
		const n = this.get(id);
		if (!n) return false;
		return n.hidden || this.ancestors(id).some((a) => a.hidden);
	}

	/** Verrou effectif : le nœud ou un de ses ancêtres est verrouillé. */
	isLocked(id: string): boolean {
		const n = this.get(id);
		if (!n) return false;
		return n.locked || this.ancestors(id).some((a) => a.locked);
	}

	/** Extrudeur effectif : celui du nœud, sinon hérité de l'ancêtre le plus proche (0 sinon). */
	effectiveExtruder(id: string): number {
		const n = this.get(id);
		if (!n) return 0;
		if (n.extruder > 0) return n.extruder;
		for (const a of this.ancestors(id)) if (a.extruder > 0) return a.extruder;
		return 0;
	}
}
