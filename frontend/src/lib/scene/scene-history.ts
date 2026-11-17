// Historique undo/redo de la scène (parité Orca Ctrl+Z / Ctrl+Y) : instantanés
// légers (structure + transformations, jamais les maillages) empilés dans
// `History`. Les maillages des objets supprimés sont retrouvés par cache ou
// rechargés via leur modèle serveur au rétablissement.
import { History } from '$lib/stores/history';
import type { SceneNode } from './objects.svelte';
import type { PlatesDocument } from './plates.svelte';
import type { SceneVolume } from './volumes';

export interface ObjectTransform {
	position: [number, number, number];
	rotation: [number, number, number];
	scale: [number, number, number];
}

/** Instantané de scène pour l'historique. */
export interface SceneSnapshot {
	nodes: SceneNode[];
	plates: PlatesDocument;
	transforms: Record<string, ObjectTransform>;
	volumes: Record<string, SceneVolume[]>;
	/** objectId → modelId (rechargement d'un objet rétabli). */
	models: Record<string, string>;
}

export interface SnapshotSource {
	nodes: readonly SceneNode[];
	plates: PlatesDocument;
	objects: readonly {
		id: string;
		position?: [number, number, number];
		rotation?: [number, number, number];
		scale?: [number, number, number];
	}[];
	volumes: Record<string, readonly SceneVolume[] | undefined>;
	models: Record<string, string>;
}

const copy3 = (v: readonly number[] | undefined, d: number): [number, number, number] =>
	v ? [v[0], v[1], v[2]] : [d, d, d];

/** Construit un instantané indépendant (copies profondes). */
export function takeSnapshot(src: SnapshotSource): SceneSnapshot {
	const transforms: Record<string, ObjectTransform> = {};
	for (const o of src.objects)
		transforms[o.id] = {
			position: copy3(o.position, 0),
			rotation: copy3(o.rotation, 0),
			scale: copy3(o.scale, 1)
		};
	const volumes: Record<string, SceneVolume[]> = {};
	for (const [id, list] of Object.entries(src.volumes))
		if (list && list.length > 0) volumes[id] = structuredClone([...list]) as SceneVolume[];
	return {
		nodes: src.nodes.map((n) => ({ ...n, settings: structuredClone(n.settings) })),
		plates: {
			plates: src.plates.plates.map((p) => ({ ...p, objectIds: [...p.objectIds] })),
			activeId: src.plates.activeId
		},
		transforms,
		volumes,
		models: { ...src.models }
	};
}

/** Clé de comparaison (deux instantanés équivalents → une seule entrée). */
export function snapshotKey(s: SceneSnapshot): string {
	return JSON.stringify(s);
}

/** Objets présents dans `next` mais absents de `prev` (à recréer) et l'inverse. */
export function objectDiff(
	prev: SceneSnapshot,
	next: SceneSnapshot
): { added: string[]; removed: string[] } {
	const a = new Set(Object.keys(prev.transforms));
	const b = new Set(Object.keys(next.transforms));
	return { added: [...b].filter((id) => !a.has(id)), removed: [...a].filter((id) => !b.has(id)) };
}

const same3 = (a: readonly number[], b: readonly number[]) =>
	a.every((v, i) => Math.abs(v - b[i]) < 1e-6);

/** Libellé d'une étape (passage de `prev` à `next`), comme les noms d'actions
 *  de l'historique d'annulation d'Orca (« Move Object », « Delete Object »…). */
export function describeChange(prev: SceneSnapshot, next: SceneSnapshot): string {
	const nodes = new Map([...prev.nodes, ...next.nodes].map((n) => [n.id, n]));
	const name = (id: string) => `« ${nodes.get(id)?.name ?? 'objet'} »`;
	const many = (verb: string, ids: string[]) =>
		ids.length === 1 ? `${verb} ${name(ids[0])}` : `${verb} ${ids.length} objets`;
	const { added, removed } = objectDiff(prev, next);
	if (added.length && !removed.length) return many('Ajouter', added);
	if (removed.length && !added.length) return many('Supprimer', removed);
	if (added.length && removed.length) return 'Remplacer des objets';
	const moved: string[] = [];
	const rotated: string[] = [];
	const scaled: string[] = [];
	for (const [id, t] of Object.entries(next.transforms)) {
		const p = prev.transforms[id];
		if (!p) continue;
		if (!same3(p.position, t.position)) moved.push(id);
		if (!same3(p.rotation, t.rotation)) rotated.push(id);
		if (!same3(p.scale, t.scale)) scaled.push(id);
	}
	if (scaled.length) return many('Mettre à l’échelle', scaled);
	if (rotated.length) return many('Pivoter', rotated);
	if (moved.length) return many('Déplacer', moved);
	const volumeIds = new Set([...Object.keys(prev.volumes), ...Object.keys(next.volumes)]);
	const volumes = [...volumeIds].filter(
		(id) => JSON.stringify(prev.volumes[id] ?? []) !== JSON.stringify(next.volumes[id] ?? [])
	);
	if (volumes.length) return many('Pièces et modificateurs de', volumes);
	const prevNodes = new Map(prev.nodes.map((n) => [n.id, n]));
	for (const n of next.nodes) {
		const p = prevNodes.get(n.id);
		if (!p) return 'Regrouper';
		if (p.name !== n.name) return `Renommer « ${p.name} » en « ${n.name} »`;
		if (p.extruder !== n.extruder) return `Extrudeur de « ${n.name} »`;
		if (JSON.stringify(p.settings) !== JSON.stringify(n.settings))
			return `Réglages de « ${n.name} »`;
		if (p.hidden !== n.hidden) return `${n.hidden ? 'Masquer' : 'Afficher'} « ${n.name} »`;
		if (p.locked !== n.locked) return `${n.locked ? 'Verrouiller' : 'Déverrouiller'} « ${n.name} »`;
		if (p.printable !== n.printable) return `Imprimable : « ${n.name} »`;
		if (p.parentId !== n.parentId) return 'Regrouper';
	}
	if (prev.nodes.length !== next.nodes.length) return 'Regrouper';
	if (prev.nodes.map((n) => n.id).join() !== next.nodes.map((n) => n.id).join())
		return 'Ordre des objets';
	if (JSON.stringify(prev.plates) !== JSON.stringify(next.plates)) return 'Plateaux';
	return 'Modification';
}

/**
 * Historique de scène : `record` ignore les états équivalents ; `suspend` sert
 * pendant une restauration (les changements qu'elle provoque ne sont pas
 * ré-enregistrés).
 */
export class SceneHistory {
	private history: History<SceneSnapshot> | null = null;
	private lastKey = '';
	private suspended = false;

	constructor(private readonly limit = 100) {}

	get canUndo(): boolean {
		return this.history?.canUndo ?? false;
	}
	get canRedo(): boolean {
		return this.history?.canRedo ?? false;
	}

	/** Point de départ (chargement du projet) : vide l'historique. */
	reset(s: SceneSnapshot): void {
		this.history = new History(s, this.limit);
		this.lastKey = snapshotKey(s);
	}

	/** Enregistre un état ; renvoie vrai si une entrée a été ajoutée. */
	record(s: SceneSnapshot): boolean {
		if (this.suspended) return false;
		const key = snapshotKey(s);
		if (!this.history) {
			this.reset(s);
			return false;
		}
		if (key === this.lastKey) return false;
		this.history.push(s);
		this.lastKey = key;
		return true;
	}

	/**
	 * Étapes nommées (liste d'historique d'Orca) : `undo[0]` = étape annulée par
	 * le prochain Ctrl+Z, `redo[0]` = étape rétablie par le prochain Ctrl+Y.
	 */
	entries(): { undo: string[]; redo: string[] } {
		if (!this.history) return { undo: [], redo: [] };
		const { past, present, future } = this.history.stacks;
		const chain = [...past, present];
		const undo: string[] = [];
		for (let i = chain.length - 1; i > 0; i--) undo.push(describeChange(chain[i - 1], chain[i]));
		const redo: string[] = [];
		let prev = present;
		for (const next of future) {
			redo.push(describeChange(prev, next));
			prev = next;
		}
		return { undo, redo };
	}

	undo(): SceneSnapshot | undefined {
		const s = this.history?.undo();
		if (s) this.lastKey = snapshotKey(s);
		return s;
	}

	redo(): SceneSnapshot | undefined {
		const s = this.history?.redo();
		if (s) this.lastKey = snapshotKey(s);
		return s;
	}

	/** Exécute `fn` sans enregistrement ; l'état final devient la référence. */
	restoring(fn: () => void, after: SceneSnapshot | (() => SceneSnapshot)): void {
		this.suspended = true;
		try {
			fn();
		} finally {
			this.suspended = false;
		}
		this.lastKey = snapshotKey(typeof after === 'function' ? after() : after);
	}
}
