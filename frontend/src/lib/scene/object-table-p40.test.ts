// Audit 40 B6 : avertissement hauteur de couche (Orca « Too large layer height »).
import { describe, expect, test } from 'vitest';
import { TABLE_COLUMNS, cellWarning, maxLayerHeight } from './object-table';

const lh = TABLE_COLUMNS.find((c) => c.key === 'layer_height')!;
const walls = TABLE_COLUMNS.find((c) => c.key === 'wall_loops')!;

describe('hauteur de couche maximale recommandée', () => {
	test('max_layer_height de l’imprimante, sinon 75 % de la buse', () => {
		expect(maxLayerHeight({ max_layer_height: ['0.28'], nozzle_diameter: ['0.4'] })).toBe(0.28);
		expect(maxLayerHeight({ max_layer_height: ['0'], nozzle_diameter: ['0.4'] })).toBe(0.3);
		expect(maxLayerHeight({})).toBeNull();
	});
	test('0,9 mm avec une buse 0,4 → avertissement ; autres colonnes jamais', () => {
		expect(cellWarning(lh, 0.9, 0.3)).not.toBeNull();
		expect(cellWarning(lh, 0.2, 0.3)).toBeNull();
		expect(cellWarning(walls, 99, 0.3)).toBeNull();
		expect(cellWarning(lh, 0.9, null)).toBeNull();
	});
});
