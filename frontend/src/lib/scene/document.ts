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
import { parseVolumes, type SceneVolume } from './volumes';
import { parseLayerRanges, type LayerRange } from './layer-ranges';

/** Version du schéma du document scène (aligné sur le backend). */
export const SCENE_SCHEMA_VERSION = 1;

/** Objet d'une scène, entièrement sérialisé. */
export interface SceneObjectDoc {
	id: string;
	modelId: string;
	/** Nom affiché (renommage) ; absent → nom du fichier du modèle. */
	name?: string;
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
	/** Volumes ajoutés (pièce, négatif, modificateur, bloqueur/forceur de supports). */
	volumes?: SceneVolume[];
	/** Plages de hauteur à réglages propres (Orca `layer_config_ranges`). */
	layerRanges?: LayerRange[];
	/** Drapeau Orca `ModelInstance::printable` (absent = true, compatibilité). */
	printable?: boolean;
	/** Nom du volume du maillage propre (`<nom>_1` après split) ; absent = `name`. */
	meshName?: string;
	/** Phase 28 — objet texte / SVG en relief ré-éditable (objects[].emboss). */
	emboss?: Record<string, unknown>;
}

/**
 * Modèles serveur connus de l'éditeur au moment de la sauvegarde (référencés
 * par un objet **ou** volontairement retirés de la scène). Permet, à la
 * réouverture, de distinguer un modèle orphelin téléversé après la dernière
 * sauvegarde (à restaurer) d'un objet supprimé puis sauvegardé (à ne pas faire
 * réapparaître). `projectId` invalide la liste quand le document a été copié
 * dans un autre projet (duplication : ids de modèles régénérés côté serveur).
 */
export interface KnownModels {
	projectId: string;
	ids: string[];
}

/** Document scène versionné (persisté dans `projects.scene`). */
export interface SceneDocument {
	schemaVersion: number;
	plates: PlatesDocument;
	objects: SceneObjectDoc[];
	knownModels?: KnownModels;
	/** Mode calibration moteur (posé par le menu Calibration, lu par le backend). */
	calibration?: Record<string, unknown>;
	/** Surcharges de presets posées par le serveur pour un projet de calibration
	 *  (POST /api/projects/{id}/calibration), appliquées au tranchage. */
	calibrationOverrides?: Record<string, unknown>;
	/** Config projet multi-filament (clés Orca : filament_colour,
	 *  flush_volumes_matrix, flush_multiplier), cf. lib/editor/multi-filament. */
	projectConfig?: Record<string, unknown>;
}

/** État vivant d'un objet (modèles riches côté client). */
export interface SceneObjectState {
	id: string;
	modelId: string;
	name?: string;
	transform: Transform;
	extruder: number;
	settings: Record<string, unknown>;
	painting: TrianglePainting;
	layerProfile: LayerBand[];
	brimEars: BrimEars;
	volumes?: SceneVolume[];
	layerRanges?: LayerRange[];
	printable: boolean;
	meshName?: string;
	emboss?: Record<string, unknown>;
}

/** Sérialise un objet vivant en document. */
export function serializeObject(o: SceneObjectState): SceneObjectDoc {
	return {
		id: o.id,
		modelId: o.modelId,
		...(o.name ? { name: o.name } : {}),
		transform: o.transform,
		extruder: o.extruder,
		settings: { ...o.settings },
		painting: o.painting.serialize(),
		layerProfile: serializeLayerProfile(o.layerProfile),
		brimEars: o.brimEars.serialize(),
		printable: o.printable,
		...(o.meshName ? { meshName: o.meshName } : {}),
		// Copie JSON : l'objet peut être un proxy réactif (non clonable par structuredClone).
		...(o.emboss ? { emboss: JSON.parse(JSON.stringify(o.emboss)) } : {}),
		...(o.volumes && o.volumes.length > 0 ? { volumes: structuredClone(o.volumes) } : {}),
		...(o.layerRanges && o.layerRanges.length > 0
			? { layerRanges: structuredClone(o.layerRanges) }
			: {})
	};
}

/** Reconstruit un objet vivant depuis un document. */
export function deserializeObject(doc: SceneObjectDoc): SceneObjectState {
	return {
		id: doc.id,
		modelId: doc.modelId,
		...(typeof doc.name === 'string' && doc.name ? { name: doc.name } : {}),
		transform: doc.transform ?? IDENTITY,
		extruder: doc.extruder ?? 0,
		settings: { ...(doc.settings ?? {}) },
		painting: TrianglePainting.deserialize(doc.painting ?? {}),
		layerProfile: deserializeLayerProfile(doc.layerProfile ?? []),
		brimEars: BrimEars.deserialize(doc.brimEars ?? { points: [] }),
		printable: doc.printable ?? true,
		...(typeof doc.meshName === 'string' && doc.meshName ? { meshName: doc.meshName } : {}),
		...(doc.emboss && typeof doc.emboss === 'object' && !Array.isArray(doc.emboss)
			? { emboss: { ...doc.emboss } }
			: {}),
		volumes: parseVolumes(doc.volumes),
		layerRanges: parseLayerRanges(doc.layerRanges)
	};
}

/** Assemble le document scène complet. */
export function serializeScene(
	plates: PlatesDocument,
	objects: SceneObjectState[],
	knownModels?: KnownModels,
	calibration?: Record<string, unknown> | null,
	projectConfig?: Record<string, unknown> | null
): SceneDocument {
	const doc: SceneDocument = {
		schemaVersion: SCENE_SCHEMA_VERSION,
		plates,
		objects: objects.map(serializeObject)
	};
	if (knownModels)
		doc.knownModels = { projectId: knownModels.projectId, ids: [...knownModels.ids] };
	if (calibration) doc.calibration = { ...calibration };
	if (projectConfig) doc.projectConfig = structuredClone(projectConfig);
	return doc;
}

/**
 * Valide et normalise un document scène quelconque (schéma inconnu → défaut).
 * Un `schemaVersion` supérieur au support est conservé tel quel (compat avant).
 */
export function parseScene(raw: unknown): SceneDocument {
	const doc = (raw ?? {}) as Partial<SceneDocument>;
	// Le backend crée les projets avec `plates: []` (tableau nu) : normalisé en
	// document `{ plates, activeId }`.
	const rawPlates = doc.plates as unknown;
	const plates: PlatesDocument = Array.isArray(rawPlates)
		? { plates: rawPlates as PlatesDocument['plates'], activeId: null }
		: rawPlates && typeof rawPlates === 'object'
			? {
					plates: Array.isArray((rawPlates as PlatesDocument).plates)
						? (rawPlates as PlatesDocument).plates
						: [],
					activeId: (rawPlates as PlatesDocument).activeId ?? null
				}
			: { plates: [], activeId: null };
	const parsed: SceneDocument = {
		schemaVersion: typeof doc.schemaVersion === 'number' ? doc.schemaVersion : SCENE_SCHEMA_VERSION,
		plates,
		objects: Array.isArray(doc.objects) ? doc.objects : []
	};
	if (doc.projectConfig && typeof doc.projectConfig === 'object')
		parsed.projectConfig = { ...doc.projectConfig };
	const known = doc.knownModels;
	if (known && typeof known.projectId === 'string' && Array.isArray(known.ids)) {
		parsed.knownModels = {
			projectId: known.projectId,
			ids: known.ids.filter((id) => typeof id === 'string')
		};
	}
	if (doc.calibration && typeof doc.calibration === 'object' && !Array.isArray(doc.calibration))
		parsed.calibration = { ...doc.calibration };
	const overrides = doc.calibrationOverrides;
	if (overrides && typeof overrides === 'object' && !Array.isArray(overrides))
		parsed.calibrationOverrides = structuredClone(overrides);
	return parsed;
}

/** Plan de restauration d'une scène à l'ouverture d'un projet. */
export interface SceneRestorePlan {
	/** Un objet par entrée du document dont le modèle existe encore (instances
	 * multiples d'un même modèle incluses), dans l'ordre du document. */
	objects: SceneObjectDoc[];
	/** Modèles du projet non référencés à restaurer comme nouveaux objets. */
	orphanModelIds: string[];
	/** Modèles distincts à récupérer/décoder (un seul fetch par modèle). */
	modelIds: string[];
}

/**
 * Calcule la restauration d'une scène : **un objet par `scene.objects`** (et
 * non un par modèle, ce qui perdait les instances partageant un modèle), puis
 * les modèles orphelins. Un orphelin n'est restauré que s'il est postérieur à
 * la dernière sauvegarde (absent de `knownModels` du même projet) ou si la
 * scène n'a jamais été sauvegardée par l'éditeur ; dans une scène sauvegardée
 * sans liste exploitable (ancien format, copie d'un autre projet), les
 * orphelins correspondent à des suppressions sauvegardées et restent masqués.
 * Pur.
 */
export function planSceneRestore(
	scene: SceneDocument,
	projectId: string,
	projectModelIds: readonly string[]
): SceneRestorePlan {
	const available = new Set(projectModelIds);
	const objects = scene.objects.filter(
		(o) => o && typeof o.modelId === 'string' && available.has(o.modelId)
	);
	const referenced = new Set(scene.objects.map((o) => o?.modelId));
	const orphans = projectModelIds.filter((id) => !referenced.has(id));
	let orphanModelIds: string[];
	if (scene.knownModels && scene.knownModels.projectId === projectId) {
		const known = new Set(scene.knownModels.ids);
		orphanModelIds = orphans.filter((id) => !known.has(id));
	} else if (scene.objects.length === 0 && scene.plates.plates.length === 0) {
		// Scène vierge (jamais sauvegardée par l'éditeur) : tout est à afficher.
		orphanModelIds = orphans;
	} else {
		orphanModelIds = [];
	}
	const modelIds = [...new Set([...objects.map((o) => o.modelId), ...orphanModelIds])];
	return { objects, orphanModelIds, modelIds };
}
