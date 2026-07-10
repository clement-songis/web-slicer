// Arbre d'objets de la scène (T053) : modèle pur pour la liste d'objets Orca —
// objets/pièces/groupes, sélection, duplication, verrouillage/masquage,
// extrudeur par objet/pièce et surcharges de réglages par objet (FR-015).
// Classe pure (pas de runes) → testable avec bun ; la couche Svelte l'observe.

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
	/** Extrudeur assigné (0 = hérité du réglage global). */
	extruder: number;
	/** Surcharges de réglages par objet (FR-015), par clé de paramètre. */
	settings: Record<string, unknown>;
}

function cloneNode(n: SceneNode): SceneNode {
	return { ...n, settings: { ...n.settings } };
}

/** Arbre d'objets avec opérations de la liste d'objets. Ordre d'insertion préservé. */
export class ObjectTree {
	private nodes: SceneNode[] = [];
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
			extruder: opts.extruder ?? 0,
			settings: { ...(opts.settings ?? {}) }
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
		const doomed = new Set([id, ...this.descendants(id).map((n) => n.id)]);
		this.nodes = this.nodes.filter((n) => !doomed.has(n.id));
		return true;
	}

	rename(id: string, name: string): void {
		const n = this.get(id);
		if (n) n.name = name;
	}

	setLocked(id: string, locked: boolean): void {
		const n = this.get(id);
		if (n) n.locked = locked;
	}

	setHidden(id: string, hidden: boolean): void {
		const n = this.get(id);
		if (n) n.hidden = hidden;
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
