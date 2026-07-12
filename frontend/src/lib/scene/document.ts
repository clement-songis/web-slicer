// Document scène complet (T060, data-model.md) : agrège l'état de préparation
// d'un projet — plateaux, objets (transformation, extrudeur, surcharges de
// réglages FR-015, peintures 3MF Orca T056, profil de hauteur de couche T058,
// oreilles de bord T057) — pour la sauvegarde/restauration versionnée.
// Sérialisation pure → testable ; le verrou optimiste (champ `version`) est géré
// côté backend (409) et surfacé par `save.ts`.
import {
	serialize as serializeLayerProfile,
	deserialize as deserializeLayerProfile,
	type LayerBand
} from './tools/layer-height';
import { TrianglePainting, type PaintDocument } from './gizmos/painting/painting';
import { BrimEars, type BrimEarsDocument } from './gizmos/brim-ears';
import { IDENTITY, type Transform } from './transform';
import type { PlatesDocument } from './plates.svelte';

/** Version du schéma du document scène (aligné sur le backend). */
export const SCENE_SCHEMA_VERSION = 1;

/** Objet d'une scène, entièrement sérialisé. */
export interface SceneObjectDoc {
	id: string;
	modelId: string;
	transform: Transform;
	/** Extrudeur assigné (0 = hérité). */
	extruder: number;
	/** Surcharges de réglages par objet (clés du registre, FR-015). */
	settings: Record<string, unknown>;
	/** Peintures par canal au format 3MF Orca. */
	painting: PaintDocument;
	/** Profil de hauteur de couche variable (vecteur plat Orca). */
	layerProfile: number[];
	/** Oreilles de bord. */
	brimEars: BrimEarsDocument;
}

/** Document scène versionné (persisté dans `projects.scene`). */
export interface SceneDocument {
	schemaVersion: number;
	plates: PlatesDocument;
	objects: SceneObjectDoc[];
}

/** État vivant d'un objet (modèles riches côté client). */
export interface SceneObjectState {
	id: string;
	modelId: string;
	transform: Transform;
	extruder: number;
	settings: Record<string, unknown>;
	painting: TrianglePainting;
	layerProfile: LayerBand[];
	brimEars: BrimEars;
}

/** Sérialise un objet vivant en document. */
export function serializeObject(o: SceneObjectState): SceneObjectDoc {
	return {
		id: o.id,
		modelId: o.modelId,
		transform: o.transform,
		extruder: o.extruder,
		settings: { ...o.settings },
		painting: o.painting.serialize(),
		layerProfile: serializeLayerProfile(o.layerProfile),
		brimEars: o.brimEars.serialize()
	};
}

/** Reconstruit un objet vivant depuis un document. */
export function deserializeObject(doc: SceneObjectDoc): SceneObjectState {
	return {
		id: doc.id,
		modelId: doc.modelId,
		transform: doc.transform ?? IDENTITY,
		extruder: doc.extruder ?? 0,
		settings: { ...(doc.settings ?? {}) },
		painting: TrianglePainting.deserialize(doc.painting ?? {}),
		layerProfile: deserializeLayerProfile(doc.layerProfile ?? []),
		brimEars: BrimEars.deserialize(doc.brimEars ?? { points: [] })
	};
}

/** Assemble le document scène complet. */
export function serializeScene(plates: PlatesDocument, objects: SceneObjectState[]): SceneDocument {
	return {
		schemaVersion: SCENE_SCHEMA_VERSION,
		plates,
		objects: objects.map(serializeObject)
	};
}

/**
 * Valide et normalise un document scène quelconque (schéma inconnu → défaut).
 * Un `schemaVersion` supérieur au support est conservé tel quel (compat avant).
 */
export function parseScene(raw: unknown): SceneDocument {
	const doc = (raw ?? {}) as Partial<SceneDocument>;
	return {
		schemaVersion: typeof doc.schemaVersion === 'number' ? doc.schemaVersion : SCENE_SCHEMA_VERSION,
		plates: doc.plates ?? { plates: [], activeId: null },
		objects: Array.isArray(doc.objects) ? doc.objects : []
	};
}
