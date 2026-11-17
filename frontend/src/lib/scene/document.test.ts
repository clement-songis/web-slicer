// Tests du document scène et de la sauvegarde (T060, analyse G5) : aller-retour
// complet (transforms, peintures, profil, oreilles, plateaux) et classification
// du conflit de verrou optimiste.
import { describe, expect, test } from 'vitest';
import {
	serializeScene,
	parseScene,
	planSceneRestore,
	serializeObject,
	deserializeObject,
	SCENE_SCHEMA_VERSION,
	type SceneObjectState
} from './document';
import { TrianglePainting, ENFORCER } from './gizmos/painting/index';
import { BrimEars } from './gizmos/brim-ears';
import { DEFAULT_PLATE_TYPE, type PlatesDocument } from './plates.svelte';
import { classifySaveError } from './save';
import { ApiError } from '../api/client';

function sampleObject(): SceneObjectState {
	const painting = new TrianglePainting();
	painting.paint('supports', [3], ENFORCER);
	const brimEars = new BrimEars();
	brimEars.add(1, 2);
	return {
		id: 'o1',
		modelId: 'm1',
		transform: { position: [1, 2, 3], rotation: [0, 90, 0], scale: [1, 1, 1] },
		extruder: 2,
		settings: { wall_loops: 3 },
		painting,
		layerProfile: [
			{ z: 0, height: 0.2 },
			{ z: 5, height: 0.1 }
		],
		brimEars,
		printable: false
	};
}

describe('document scène', () => {
	test('aller-retour d’un objet préserve tout l’état', () => {
		const doc = serializeObject(sampleObject());
		// Passe par JSON comme lors d’un PUT/GET réel.
		const round = deserializeObject(JSON.parse(JSON.stringify(doc)));
		expect(round.transform.position).toEqual([1, 2, 3]);
		expect(round.extruder).toBe(2);
		expect(round.settings.wall_loops).toBe(3);
		expect(round.painting.get('supports', 3)).toBe(ENFORCER);
		expect(round.layerProfile).toEqual([
			{ z: 0, height: 0.2 },
			{ z: 5, height: 0.1 }
		]);
		expect(round.brimEars.list()[0]).toEqual({ x: 1, y: 2 });
		expect(round.printable).toBe(false);
		expect(deserializeObject({ ...doc, printable: undefined }).printable).toBe(true);
	});

	test('meshName (volume du maillage propre après split) : aller-retour, absent par défaut', () => {
		const plain = serializeObject(sampleObject());
		expect('meshName' in plain).toBe(false);
		expect(deserializeObject(plain).meshName).toBeUndefined();
		const split = serializeObject({ ...sampleObject(), meshName: 'pair.stl_1' });
		expect(split.meshName).toBe('pair.stl_1');
		expect(deserializeObject(JSON.parse(JSON.stringify(split))).meshName).toBe('pair.stl_1');
	});

	test('serializeScene inclut version de schéma, plateaux et objets', () => {
		// Document plateaux littéral (le modèle réactif `PlateSet` est testé sous
		// vitest ; ici on reste sous bun avec une donnée pure équivalente).
		const plates: PlatesDocument = {
			plates: [{ id: 'plate-1', name: 'Plateau 1', plateType: DEFAULT_PLATE_TYPE, objectIds: [] }],
			activeId: 'plate-1'
		};
		const scene = serializeScene(plates, [sampleObject()]);
		expect(scene.schemaVersion).toBe(SCENE_SCHEMA_VERSION);
		expect(scene.objects.length).toBe(1);
		expect(scene.plates.plates.length).toBe(1);
	});

	test('parseScene normalise un document inconnu', () => {
		expect(parseScene(null)).toEqual({
			schemaVersion: SCENE_SCHEMA_VERSION,
			plates: { plates: [], activeId: null },
			objects: []
		});
		expect(parseScene({ schemaVersion: 2, objects: 'oops' }).objects).toEqual([]);
		expect(parseScene({ schemaVersion: 2 }).schemaVersion).toBe(2); // compat avant
	});
});

describe('parseScene / knownModels', () => {
	test('normalise le tableau de plateaux nu du backend', () => {
		const scene = parseScene({ schema_version: 1, objects: [], plates: [] });
		expect(scene.plates).toEqual({ plates: [], activeId: null });
	});

	test('aller-retour de knownModels via JSON', () => {
		const plates: PlatesDocument = { plates: [], activeId: null };
		const scene = serializeScene(plates, [sampleObject()], { projectId: 'p1', ids: ['m1', 'm2'] });
		const back = parseScene(JSON.parse(JSON.stringify(scene)));
		expect(back.knownModels).toEqual({ projectId: 'p1', ids: ['m1', 'm2'] });
	});
});

describe('planSceneRestore', () => {
	const plates: PlatesDocument = {
		plates: [{ id: 'plate-1', name: 'Plateau 1', plateType: DEFAULT_PLATE_TYPE, objectIds: [] }],
		activeId: 'plate-1'
	};
	function obj(id: string, modelId: string) {
		return { ...serializeObject(sampleObject()), id, modelId };
	}

	test('restaure une entrée par objet, y compris les instances d’un même modèle', () => {
		const scene = parseScene({
			schemaVersion: 1,
			plates,
			objects: [obj('o1', 'm1'), obj('o2', 'm1'), obj('o3', 'm2')],
			knownModels: { projectId: 'p1', ids: ['m1', 'm2'] }
		});
		const plan = planSceneRestore(scene, 'p1', ['m1', 'm2']);
		expect(plan.objects.map((o) => o.id)).toEqual(['o1', 'o2', 'o3']);
		// Un seul fetch/décodage par modèle.
		expect(plan.modelIds).toEqual(['m1', 'm2']);
		expect(plan.orphanModelIds).toEqual([]);
	});

	test('ignore les objets dont le modèle n’existe plus', () => {
		const scene = parseScene({ schemaVersion: 1, plates, objects: [obj('o1', 'gone')] });
		expect(planSceneRestore(scene, 'p1', ['m1']).objects).toEqual([]);
	});

	test('ne fait pas réapparaître un objet supprimé puis sauvegardé', () => {
		const scene = parseScene({
			schemaVersion: 1,
			plates,
			objects: [obj('o1', 'm1')],
			knownModels: { projectId: 'p1', ids: ['m1', 'm-supprime'] }
		});
		expect(planSceneRestore(scene, 'p1', ['m1', 'm-supprime']).orphanModelIds).toEqual([]);
	});

	test('restaure un modèle téléversé après la dernière sauvegarde', () => {
		const scene = parseScene({
			schemaVersion: 1,
			plates,
			objects: [obj('o1', 'm1')],
			knownModels: { projectId: 'p1', ids: ['m1'] }
		});
		const plan = planSceneRestore(scene, 'p1', ['m1', 'm-nouveau']);
		expect(plan.orphanModelIds).toEqual(['m-nouveau']);
		expect(plan.modelIds).toEqual(['m1', 'm-nouveau']);
	});

	test('scène vierge (jamais sauvegardée) : tous les modèles sont restaurés', () => {
		const scene = parseScene({ schema_version: 1, objects: [], plates: [] });
		expect(planSceneRestore(scene, 'p1', ['m1', 'm2']).orphanModelIds).toEqual(['m1', 'm2']);
	});

	test('scène sauvegardée sans liste exploitable : les orphelins restent masqués', () => {
		// Ancien format (pas de knownModels).
		const legacy = parseScene({ schemaVersion: 1, plates, objects: [obj('o1', 'm1')] });
		expect(planSceneRestore(legacy, 'p1', ['m1', 'm2']).orphanModelIds).toEqual([]);
		// Copie d’un autre projet : la liste référence d’anciens ids.
		const copied = parseScene({
			schemaVersion: 1,
			plates,
			objects: [obj('o1', 'n1')],
			knownModels: { projectId: 'source', ids: ['m1', 'm2'] }
		});
		expect(planSceneRestore(copied, 'copie', ['n1', 'n2']).orphanModelIds).toEqual([]);
	});
});

describe('classifySaveError', () => {
	test('un 409 est un conflit de verrou optimiste', () => {
		expect(classifySaveError(new ApiError(409, 'conflict', 'décalé'))).toBe('conflict');
	});

	test('les autres erreurs ne sont pas des conflits', () => {
		expect(classifySaveError(new ApiError(500, 'error', 'boom'))).toBe('error');
		expect(classifySaveError(new Error('réseau'))).toBe('error');
	});
});

describe('surcharges de calibration du serveur', () => {
	test('parseScene conserve scene.calibrationOverrides (copie)', () => {
		const raw = { plates: [], objects: [], calibrationOverrides: { spiral_mode: true } };
		const parsed = parseScene(raw);
		expect(parsed.calibrationOverrides).toEqual({ spiral_mode: true });
		expect(parsed.calibrationOverrides).not.toBe(raw.calibrationOverrides);
		expect(parseScene({ calibrationOverrides: [1] }).calibrationOverrides).toBeUndefined();
	});
});
