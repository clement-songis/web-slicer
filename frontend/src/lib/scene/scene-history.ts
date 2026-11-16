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
