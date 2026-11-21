import { describe, expect, it } from 'vitest';
import { ENFORCER, BLOCKER } from './facets';
import { ERASE, mouseStrokeState } from './mouse-state';

describe('mouseStrokeState (Orca GLGizmoPainterBase::gizmo_event)', () => {
	it('supports / couture : gauche = état choisi, droit = bloquer, Maj = effacer', () => {
		for (const ch of ['supports', 'seam'] as const) {
			expect(mouseStrokeState(ch, 0, false, ENFORCER)).toBe(ENFORCER);
			expect(mouseStrokeState(ch, 2, false, ENFORCER)).toBe(BLOCKER);
			expect(mouseStrokeState(ch, 0, true, ENFORCER)).toBe(ERASE);
			expect(mouseStrokeState(ch, 2, true, ENFORCER)).toBe(ERASE);
		}
	});
	it('surface floue : droit = gomme', () => {
		expect(mouseStrokeState('fuzzy', 0, false, ENFORCER)).toBe(ENFORCER);
		expect(mouseStrokeState('fuzzy', 2, false, ENFORCER)).toBe(ERASE);
	});
	it('multi-matériaux : gauche = filament choisi, Maj = effacer, droit non géré', () => {
		expect(mouseStrokeState('mmu', 0, false, 3)).toBe(3);
		expect(mouseStrokeState('mmu', 0, true, 3)).toBe(ERASE);
		expect(mouseStrokeState('mmu', 2, false, 3)).toBeNull();
	});
	it('bouton du milieu : jamais de peinture', () => {
		expect(mouseStrokeState('supports', 1, false, ENFORCER)).toBeNull();
	});
});
