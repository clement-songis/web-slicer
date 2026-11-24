import { beforeAll, afterAll, describe, expect, test } from 'vitest';
import { setLocale } from '#lib/i18n';
import {
	PART_TYPE_CHOICES,
	changeVolumeType,
	parseCutInfo,
	parseVolumes,
	partTypeChoices,
	type SceneVolume
} from './volumes';
import { deserializeObject, serializeObject } from './document';

const vol: SceneVolume = {
	id: 'v',
	type: 'modifier',
	name: 'Modificateur 1',
	shape: 'box',
	size: [5, 5, 5],
	offset: [0, 0, 0],
	settings: { wall_loops: 4 },
	extruder: 2
};

describe('« Changer le type » d’une pièce (change_part_type d’Orca)', () => {
	test('5 types (noms de l’application), relief limité à pièce / négatif / modificateur', () => {
		expect(PART_TYPE_CHOICES.map((c) => c.label)).toEqual([
			'Pièce',
			'Volume négatif',
			'Modificateur',
			'Bloqueur de supports',
			'Forceur de supports'
		]);
		expect(PART_TYPE_CHOICES.map((c) => c.type)).toEqual([
			'part',
			'negative',
			'modifier',
			'blocker',
			'enforcer'
		]);
		expect(partTypeChoices({ emboss: { type: 'text' } }).map((c) => c.type)).toEqual([
			'part',
			'negative',
			'modifier'
		]);
	});

	test('type changé, réglages et extrudeur conservés (set_type) ; inchangé ou interdit → null', () => {
		const next = changeVolumeType(vol, 'negative')!;
		expect(next.type).toBe('negative');
		expect(next.settings).toEqual({ wall_loops: 4 });
		expect(next.extruder).toBe(2);
		expect(vol.type).toBe('modifier');
		expect(changeVolumeType(vol, 'modifier')).toBeNull();
		expect(changeVolumeType({ ...vol, emboss: { type: 'text' } }, 'blocker')).toBeNull();
	});
});

describe('informations de coupe dans le document scène', () => {
	test('cutInfo des volumes lu / gardé, forme invalide écartée', () => {
		expect(
			parseCutInfo({ connectorType: 'dowel', radiusTolerance: 0.1, heightTolerance: 0.2 })
		).toEqual({
			connectorType: 'dowel',
			radiusTolerance: 0.1,
			heightTolerance: 0.2
		});
		expect(parseCutInfo({ connectorType: 'nail' })).toBeNull();
		const [v] = parseVolumes([
			{ ...vol, cutInfo: { connectorType: 'plug', radiusTolerance: 0, heightTolerance: 0.1 } }
		]);
		expect(v.cutInfo?.connectorType).toBe('plug');
	});

	test('cutId d’objet sérialisé et relu', () => {
		const doc = {
			id: 'o',
			modelId: 'm',
			transform: { position: [0, 0, 0], rotation: [0, 0, 0], scale: [1, 1, 1] },
			extruder: 0,
			settings: {},
			painting: {},
			layerProfile: [],
			brimEars: { points: [] },
			cutId: { id: 3, checkSum: 2, connectorsCnt: 1 }
		} as unknown as Parameters<typeof deserializeObject>[0];
		const live = deserializeObject(doc);
		expect(live.cutId).toEqual({ id: 3, checkSum: 2, connectorsCnt: 1 });
		expect(serializeObject(live).cutId).toEqual({ id: 3, checkSum: 2, connectorsCnt: 1 });
		expect(serializeObject({ ...live, cutId: null }).cutId).toBeUndefined();
	});
});

// Les libellés de ces modules sont traduits (tr) : valeurs attendues en français.
beforeAll(() => setLocale('fr'));
afterAll(() => setLocale('en'));
