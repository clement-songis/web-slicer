import { describe, expect, it } from 'vitest';
import { autoFlushResult, changedSlots, mergeFlushSlots } from './auto-flush';

const s = (colours: string[], filaments: string[]) => ({ colours, filaments });

describe('purge automatique (Orca auto_calculate_flush)', () => {
	it('slots changés selon le mode', () => {
		const a = s(['#ff0000', '#00ff00'], ['p1', 'p2']);
		expect(changedSlots(a, s(['#FF0000', '#0000ff'], ['p1', 'p2']), 'all')).toEqual([1]);
		expect(changedSlots(a, s(['#ff0000', '#00ff00'], ['p1', 'p3']), 'all')).toEqual([1]);
		expect(changedSlots(a, s(['#ff0000', '#00ff00'], ['p1', 'p3']), 'color')).toEqual([]);
		expect(changedSlots(a, s(['#000000', '#00ff00'], ['p1', 'p2']), 'disabled')).toEqual([]);
		expect(changedSlots(null, a, 'all')).toEqual([]);
		expect(changedSlots(a, s(['#ff0000', '#00ff00', '#fff'], ['p1', 'p2', 'p3']), 'all')).toEqual(
			[]
		);
	});
	it('fusion : lignes/colonnes recalculées, autres valeurs conservées', () => {
		const current = [0, 1, 2, 3, 0, 5, 6, 7, 0];
		const computed = [0, 10, 20, 30, 0, 50, 60, 70, 0];
		expect(mergeFlushSlots(current, computed, 3, [1])).toEqual([0, 10, 2, 30, 0, 50, 6, 70, 0]);
		expect(mergeFlushSlots(undefined, computed, 3, [1])).toEqual(computed);
		expect(mergeFlushSlots(current, [1, 2], 3, [1])).toEqual(current);
	});
	it('saisie manuelle conservée', () => {
		// Saisie dans le dialogue puis « Appliquer » : couleurs et filaments
		// inchangés → aucun recalcul.
		const snap = s(['#ffffff', '#e11d48'], ['p1', 'p1']);
		expect(changedSlots(snap, s(['#ffffff', '#e11d48'], ['p1', 'p1']), 'all')).toEqual([]);
		// Recalcul lancé avant la saisie, reçu après : la saisie l'emporte.
		const started = { matrix: undefined, multiplier: undefined };
		const manual = { matrix: [0, 150, 280, 0], multiplier: 1.2 };
		expect(autoFlushResult(started, manual, [0, 92, 249, 0], 2, [1])).toBeNull();
		// Multiplicateur seul modifié : saisie aussi.
		expect(autoFlushResult(started, { multiplier: 1.2 }, [0, 92, 249, 0], 2, [1])).toBeNull();
		// Sans saisie : ligne/colonne du slot changé recalculées, valeurs saisies
		// ailleurs conservées, multiplicateur hors du résultat (jamais modifié).
		const cur = { matrix: [0, 150, 280, 300, 0, 120, 90, 60, 0], multiplier: 1.2 };
		expect(
			autoFlushResult(
				cur,
				{ ...cur, matrix: [...cur.matrix] },
				[0, 10, 20, 30, 0, 50, 60, 70, 0],
				3,
				[2]
			)
		).toEqual([0, 150, 20, 300, 0, 50, 60, 70, 0]);
		expect(autoFlushResult(started, started, [0, 91.6, 249.2, 0], 2, [1])).toEqual([0, 92, 249, 0]);
		expect(autoFlushResult(started, started, [0, 1, 2], 2, [1])).toBeNull();
		expect(autoFlushResult(started, started, [0, 1, 2, 0], 2, [])).toBeNull();
	});
});
