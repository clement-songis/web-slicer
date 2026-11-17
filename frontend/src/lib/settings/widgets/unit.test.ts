import { describe, expect, it } from 'vitest';
import { PARAMS } from '../../../generated/params';
import { unitText } from './types';

describe('unité affichée (sidetext)', () => {
	it('garde l unité, retire une infobulle collée', () => {
		expect(unitText('mm')).toBe('mm');
		expect(unitText(PARAMS.top_shell_layers.sidetext)).toBe('layers');
		expect(unitText(PARAMS.inner_wall_speed.sidetext)).toBe('mm/s');
		expect(unitText(null)).toBe('');
	});
	it('aucune unité affichée de plus de 12 caractères dans le registre', () => {
		for (const def of Object.values(PARAMS))
			expect(unitText(def.sidetext).length, def.key).toBeLessThanOrEqual(12);
	});
});
