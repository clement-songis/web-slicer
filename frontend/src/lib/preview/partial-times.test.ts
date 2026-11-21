import { describe, expect, test } from 'vitest';
import { hasEvents, partialTimes } from './partial-times';

describe('événements du résumé (Orca generate_partial_times)', () => {
	test('G-code externe : pause puis tronçon final (pas de changement de couleur)', () => {
		// Phase 36 : « Pause à 4m 50s (reste 9m 55s) » et « Changement de couleur à
		// 5m 5s » ; le second est le tronçon final marqué ColorChange par Orca.
		const items = partialTimes(
			[
				{ type: 'pause_print', time: 290, remaining: 595 },
				{ type: 'color_change', time: 305, remaining: 305 }
			],
			[
				{ type: 'custom', extruder: 1, color: '' },
				{ type: 'pause_print', extruder: 1, color: '' },
				{ type: 'custom', extruder: 1, color: '' }
			]
		);
		expect(items.map((i) => i.kind)).toEqual(['print', 'pause', 'print']);
		expect(items[0]).toMatchObject({ duration: 290, remaining: 595 });
		expect(items[2]).toMatchObject({ duration: 305, remaining: 305 });
		expect(hasEvents(items)).toBe(true);
	});

	test('changement de couleur apparié, puis tronçon final', () => {
		const items = partialTimes(
			[
				{ type: 'color_change', time: 100, remaining: 300 },
				{ type: 'color_change', time: 200, remaining: 200 }
			],
			[{ type: 'color_change', extruder: 2, color: '#FF0000' }]
		);
		expect(items.map((i) => i.kind)).toEqual(['print', 'color_change', 'print']);
		expect(items[1]).toMatchObject({ extruder: 2, color: '#FF0000' });
		expect(items[2].extruder).toBe(2);
	});

	test('aucun événement réel : rien à afficher', () => {
		const items = partialTimes([{ type: 'color_change', time: 50, remaining: 50 }], []);
		expect(items).toHaveLength(1);
		expect(hasEvents(items)).toBe(false);
		expect(partialTimes([{ type: 'pause_print', time: 5, remaining: 9 }], [])).toEqual([]);
	});
});
