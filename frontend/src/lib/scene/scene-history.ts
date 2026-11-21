// Historique undo/redo de la scène (parité Orca Ctrl+Z / Ctrl+Y) : instantanés
// légers (structure + transformations, jamais les maillages) empilés dans
// `History`. Les maillages des objets supprimés sont retrouvés par cache ou
// rechargés via leur modèle serveur au rétablissement.
import { History } from '$lib/stores/history';
import type { SceneNode } from './objects.svelte';
import type { PlatesDocument } from './plates.svelte';
import type { SceneVolume } from './volumes';
import { tr } from '$lib/i18n';

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
	const name = (id: string) => nodes.get(id)?.name ?? tr('object');
	// Clés i18n : « Verbe “{name}” » (un objet) / « Verbe {n} objects » (plusieurs).
	const many = (one: string, several: string, ids: string[]) =>
		ids.length === 1 ? tr(one, { name: name(ids[0]) }) : tr(several, { n: ids.length });
	const { added, removed } = objectDiff(prev, next);
	if (added.length && !removed.length) return many('Add “{name}”', 'Add {n} objects', added);
	if (removed.length && !added.length)
		return many('Delete “{name}”', 'Delete {n} objects', removed);
	if (added.length && removed.length) return tr('Replace objects');
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
	if (scaled.length) return many('Scale “{name}”', 'Scale {n} objects', scaled);
	if (rotated.length) return many('Rotate “{name}”', 'Rotate {n} objects', rotated);
	if (moved.length) return many('Move “{name}”', 'Move {n} objects', moved);
	const volumeIds = new Set([...Object.keys(prev.volumes), ...Object.keys(next.volumes)]);
	const volumes = [...volumeIds].filter(
		(id) => JSON.stringify(prev.volumes[id] ?? []) !== JSON.stringify(next.volumes[id] ?? [])
	);
	if (volumes.length)
		return many('Parts and modifiers of “{name}”', 'Parts and modifiers of {n} objects', volumes);
	const prevNodes = new Map(prev.nodes.map((n) => [n.id, n]));
	for (const n of next.nodes) {
		const p = prevNodes.get(n.id);
		if (!p) return tr('Regroup');
		if (p.name !== n.name) return tr('Rename “{from}” to “{to}”', { from: p.name, to: n.name });
		if (p.extruder !== n.extruder) return tr('Extruder of “{name}”', { name: n.name });
		if (JSON.stringify(p.settings) !== JSON.stringify(n.settings))
			return tr('Settings of “{name}”', { name: n.name });
		if (p.hidden !== n.hidden)
			return tr(n.hidden ? 'Hide “{name}”' : 'Show “{name}”', { name: n.name });
		if (p.locked !== n.locked)
			return tr(n.locked ? 'Lock “{name}”' : 'Unlock “{name}”', { name: n.name });
		if (p.printable !== n.printable) return tr('Printable: “{name}”', { name: n.name });
		if (p.parentId !== n.parentId) return tr('Regroup');
	}
	if (prev.nodes.length !== next.nodes.length) return tr('Regroup');
	if (prev.nodes.map((n) => n.id).join() !== next.nodes.map((n) => n.id).join())
		return tr('Object order');
	if (JSON.stringify(prev.plates) !== JSON.stringify(next.plates)) return tr('Plates');
	return tr('Modification');
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
