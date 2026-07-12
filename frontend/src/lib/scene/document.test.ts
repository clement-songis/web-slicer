// Tests du document scène et de la sauvegarde (T060, analyse G5) : aller-retour
// complet (transforms, peintures, profil, oreilles, plateaux) et classification
// du conflit de verrou optimiste.
import { describe, expect, test } from 'bun:test';
import {
	serializeScene,
	parseScene,
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
		brimEars
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

describe('classifySaveError', () => {
	test('un 409 est un conflit de verrou optimiste', () => {
		expect(classifySaveError(new ApiError(409, 'conflict', 'décalé'))).toBe('conflict');
	});

	test('les autres erreurs ne sont pas des conflits', () => {
		expect(classifySaveError(new ApiError(500, 'error', 'boom'))).toBe('error');
		expect(classifySaveError(new Error('réseau'))).toBe('error');
	});
});
