import { afterAll, beforeAll, describe, expect, test } from 'vitest';
import { setLocale } from '#lib/i18n';
import {
	TABLE_COLUMNS,
	cellText,
	cellValue,
	parseCell,
	sortRows,
	tableRows,
	visibleColumns
} from './object-table';

beforeAll(() => setLocale('fr'));
afterAll(() => setLocale('en'));

const col = (key: string) => TABLE_COLUMNS.find((c) => c.key === key)!;
const objects = [
	{
		id: 'b',
		name: 'Boîte',
		plate: 2,
		printable: true,
		extruder: 2,
		settings: { wall_loops: 4 },
		volumes: [
			{
				id: 'm',
				name: 'Modificateur 1',
				type: 'modifier',
				settings: { sparse_infill_density: '40%' }
			},
			{ id: 'n', name: 'Négatif', type: 'negative', settings: {} }
		]
	},
	{ id: 'a', name: 'arche', plate: 1, printable: false, extruder: 0, settings: {} }
];

describe('tableau des paramètres d’objets (ObjectGridTable d’Orca)', () => {
	test('colonnes d’Orca, filtrées par catégorie (les colonnes « Tout » restent)', () => {
		expect(TABLE_COLUMNS.map((c) => c.key)).toEqual([
			'printable',
			'plate_index',
			'name',
			'extruder',
			'layer_height',
			'wall_loops',
			'sparse_infill_density',
			'enable_support',
			'brim_type',
			'outer_wall_speed'
		]);
		expect(visibleColumns('speed').map((c) => c.key)).toEqual([
			'printable',
			'plate_index',
			'name',
			'extruder',
			'outer_wall_speed'
		]);
	});

	test('lignes : objets puis pièces / modificateurs (négatifs exclus)', () => {
		const rows = tableRows(objects);
		expect(rows.map((r) => [r.kind, r.name])).toEqual([
			['object', 'Boîte'],
			['volume', 'Modificateur 1'],
			['object', 'arche']
		]);
	});

	test('tri par plateau / nom / filament : volumes gardés sous leur objet', () => {
		const rows = tableRows(objects);
		expect(sortRows(rows, 'plate_index').map((r) => r.name)).toEqual([
			'arche',
			'Boîte',
			'Modificateur 1'
		]);
		expect(sortRows(rows, 'name', true).map((r) => r.name)).toEqual([
			'Boîte',
			'Modificateur 1',
			'arche'
		]);
		expect(sortRows(rows, 'name').map((r) => r.name)).toEqual(['arche', 'Boîte', 'Modificateur 1']);
		expect(sortRows(rows, 'extruder', true)[0].name).toBe('Boîte');
	});

	test('valeur : surcharge, sinon objet parent (volume), sinon profil', () => {
		const rows = tableRows(objects);
		const inherited = { wall_loops: 2, sparse_infill_density: '15%' };
		expect(cellValue(rows[0], col('wall_loops'), inherited)).toEqual({
			value: 4,
			overridden: true,
			inherited: 2
		});
		expect(cellValue(rows[1], col('wall_loops'), inherited, objects[0].settings)).toMatchObject({
			value: 4,
			overridden: false
		});
		expect(cellValue(rows[1], col('sparse_infill_density'), inherited)).toMatchObject({
			value: '40%',
			overridden: true
		});
		expect(cellValue(rows[2], col('wall_loops'), { wall_loops: [3] })).toMatchObject({ value: 3 });
	});

	test('saisie validée comme PrintConfig (bornes) et forme Orca', () => {
		expect(parseCell(col('wall_loops'), '3')).toBe(3);
		expect(parseCell(col('wall_loops'), '2.5')).toBeNull();
		expect(parseCell(col('sparse_infill_density'), '25')).toBe('25%');
		expect(parseCell(col('sparse_infill_density'), '25 %'.replace(' ', ''))).toBe('25%');
		expect(parseCell(col('sparse_infill_density'), '120')).toBeNull();
		expect(parseCell(col('layer_height'), '0,16')).toBe(0.16);
		expect(parseCell(col('layer_height'), '0')).toBeNull();
		expect(parseCell(col('enable_support'), true)).toBe(true);
		expect(parseCell(col('brim_type'), 'no_brim')).toBe('no_brim');
		expect(parseCell(col('brim_type'), 'nope')).toBeNull();
		expect(cellText(col('sparse_infill_density'), '15%')).toBe('15');
		expect(cellText(col('brim_type'), 'outer_only')).toBe('Bordure extérieure uniquement');
	});
});
