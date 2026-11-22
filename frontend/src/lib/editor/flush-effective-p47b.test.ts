import { describe, expect, it } from 'vitest';
import { effectiveFlushView } from './auto-flush';

describe('effectiveFlushView (POST /api/flush-volumes effective, phase 47b)', () => {
	it('matrice calculée (1 buse) → source auto, arrondie', () => {
		expect(
			effectiveFlushView({ matrix: [0, 249.6, 120.2, 0], source: 'computed', nozzles: 1 }, 2)
		).toEqual({ base: [0, 250, 120, 0], tail: [], source: 'auto' });
	});

	it('matrice du projet sur 2 buses → 1re buse éditée, 2e conservée', () => {
		expect(
			effectiveFlushView({ matrix: [0, 1, 2, 0, 0, 3, 4, 0], source: 'project', nozzles: 2 }, 2)
		).toEqual({ base: [0, 1, 2, 0], tail: [0, 3, 4, 0], source: 'project' });
	});

	it('taille incohérente (couleurs locales ≠ projet enregistré) → null', () => {
		expect(
			effectiveFlushView({ matrix: [0, 1, 2, 0], source: 'computed', nozzles: 1 }, 3)
		).toBeNull();
		expect(effectiveFlushView({ matrix: [0, -1, 2, 0], nozzles: 1 }, 2)).toBeNull();
		expect(effectiveFlushView(null, 2)).toBeNull();
		expect(effectiveFlushView({ matrix: [], nozzles: 1 }, 1)).toBeNull();
	});
});
