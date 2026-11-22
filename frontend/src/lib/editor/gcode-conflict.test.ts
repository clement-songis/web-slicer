// Phase 46 (audit 44 CONFL-a) : avertissement « Conflicts of G-code paths » d'Orca.
import { describe, expect, it } from 'vitest';
import { conflictMessage, parseConflict } from './gcode-conflict';

describe('gcode-conflict', () => {
	it('lit les formes du résultat serveur', () => {
		expect(
			parseConflict({ layer: 3, z: 0.8, object1: 'cube20.stl', object2: 'cylinder.stl' })
		).toEqual({
			layer: 3,
			z: 0.8,
			object1: 'cube20.stl',
			object2: 'cylinder.stl'
		});
		expect(parseConflict({ layer: '3', height: '0.8', obj1: 'a', obj2: 'b' })).toMatchObject({
			layer: 3,
			z: 0.8
		});
		expect(parseConflict(null)).toBeNull();
		expect(parseConflict({ layer: 1, z: 0.2, object1: 'a' })).toBeNull();
	});
	it('message d’Orca, Z à 2 décimales', () => {
		expect(conflictMessage({ layer: 3, z: 0.8, object1: 'a', object2: 'b' })).toBe(
			'Conflicts of G-code paths have been found at layer 3, Z = 0.80mm. Please separate the conflicted objects farther (a <-> b).'
		);
	});
});
