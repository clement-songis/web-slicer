// Phase 41 (lot 40, D1) : surcharges non enregistrées du projet
// (`active_presets.overrides`) — lecture / écriture, retrait des clés
// enregistrées, « Abandonner », slots filament, brouillon de tranchage.
import { describe, expect, test } from 'vitest';
import {
	applyOverrides,
	coerceOverrideValue,
	compactOverrides,
	emptyActivePresets,
	overridesOf,
	parseActivePresets,
	pendingAfterSlotRemoval,
	pendingOverridesFor,
	serializeActivePresets,
	withPendingFilament,
	withoutOverrides,
	withoutPending,
	type ActivePresets
} from './presets';
import { buildProjectOverrides, type OverridesInput } from './slice-draft';

describe('parse / serialize : active_presets.overrides', () => {
	test('lit les surcharges (format SliceDraft) et les réécrit', () => {
		const raw = {
			printer: 'm',
			process: 'p',
			filaments: ['f1', 'f2'],
			overrides: { process: { wall_loops: 5 }, filaments: [null, { nozzle_temperature: [215] }] }
		};
		const a = parseActivePresets(raw);
		expect(a.overrides).toEqual({
			process: { wall_loops: 5 },
			filaments: [null, { nozzle_temperature: [215] }]
		});
		expect(serializeActivePresets(a)).toEqual(raw);
	});

	test('surcharges vides ou illisibles → absentes (sélection seule)', () => {
		for (const overrides of [{}, { process: {} }, { filaments: [null, {}] }, 'x', null]) {
			const a = parseActivePresets({ process: 'p', overrides });
			expect(a).toEqual({ printer: null, process: 'p', filaments: [] });
			expect(serializeActivePresets(a)).toEqual({ process: 'p' });
		}
	});

	test('compactage du serveur : null de fin tronqués, clés triées', () => {
		const o = compactOverrides({
			printer: { z: 1, a: 2 },
			filaments: [{ x: 1 }, null, {}]
		});
		expect(o).toEqual({ printer: { a: 2, z: 1 }, filaments: [{ x: 1 }] });
		expect(Object.keys(o!.printer!)).toEqual(['a', 'z']);
	});

	test('omettre overrides efface les modifications (document remplacé en bloc)', () => {
		const a: ActivePresets = { printer: 'm', process: 'p', filaments: [], overrides: undefined };
		expect('overrides' in serializeActivePresets(a)).toBe(false);
	});
});

describe('retrait des clés enregistrées / Abandonner', () => {
	const o = {
		process: { wall_loops: 5, sparse_infill_density: '20%' },
		printer: { retraction_length: [1] },
		filaments: [{ a: 1 }, { b: 2 }]
	};

	test('Enregistrer sous : seules les clés enregistrées quittent les surcharges', () => {
		expect(withoutOverrides(o, 'process', 0, ['wall_loops'])).toEqual({
			...o,
			process: { sparse_infill_density: '20%' }
		});
		expect(withoutOverrides(o, 'filament', 1, ['b'])).toEqual({ ...o, filaments: [{ a: 1 }] });
	});

	test("Abandonner : l'entrée du type disparaît", () => {
		expect(withoutOverrides(o, 'machine')).toEqual({ process: o.process, filaments: o.filaments });
		expect(withoutOverrides(o, 'filament', 0)).toEqual({ ...o, filaments: [null, { b: 2 }] });
		expect(withoutOverrides({ process: { a: 1 } }, 'process')).toBeUndefined();
	});

	test('overridesOf : badge « (modifié) » ⇔ entrée non vide', () => {
		expect(overridesOf(o, 'process')).toEqual(o.process);
		expect(overridesOf(o, 'filament', 1)).toEqual({ b: 2 });
		expect(overridesOf(o, 'filament', 4)).toBeNull();
		expect(overridesOf(undefined, 'machine')).toBeNull();
	});
});

describe('surcharges en attente (non encore chargées)', () => {
	const pending: ActivePresets = {
		printer: 'm',
		process: 'p',
		filaments: ['f1', 'f2'],
		overrides: { process: { wall_loops: 5 }, filaments: [null, { x: 1 }] }
	};

	test('reprises seulement pour le preset auquel elles appartiennent', () => {
		expect(pendingOverridesFor(pending, 'process', 0, 'p')).toEqual({ wall_loops: 5 });
		expect(pendingOverridesFor(pending, 'process', 0, 'autre')).toBeNull();
		expect(pendingOverridesFor(pending, 'filament', 1, 'f2')).toEqual({ x: 1 });
		expect(pendingOverridesFor(pending, 'filament', 1, 'f1')).toBeNull();
	});

	test('consommées / mises en attente / slot retiré', () => {
		expect(withoutPending(pending, 'process').overrides).toEqual({ filaments: [null, { x: 1 }] });
		const stashed = withPendingFilament(pending, 0, 'f1', { y: 2 });
		expect(stashed.overrides?.filaments).toEqual([{ y: 2 }, { x: 1 }]);
		const far = withPendingFilament(emptyActivePresets(), 2, 'f3', { z: 3 });
		expect(far.filaments).toEqual(['', '', 'f3']);
		expect(pendingOverridesFor(far, 'filament', 2, 'f3')).toEqual({ z: 3 });
		const removed = pendingAfterSlotRemoval(pending, 0);
		expect(removed.filaments).toEqual(['f2']);
		expect(pendingOverridesFor(removed, 'filament', 0, 'f2')).toEqual({ x: 1 });
	});
});

describe('valeurs typées comme le preset résolu', () => {
	test('chaînes Orca → forme de /resolved', () => {
		expect(coerceOverrideValue(3, '5')).toBe(5);
		expect(coerceOverrideValue(true, '0')).toBe(false);
		expect(coerceOverrideValue([0.4], '0.6,0.6')).toEqual([0.6, 0.6]);
		expect(coerceOverrideValue('15%', '20%')).toBe('20%');
		expect(coerceOverrideValue(0.2, '50%')).toBe('50%');
		expect(coerceOverrideValue(['a'], 'b')).toBe('b');
		expect(coerceOverrideValue(3, 7)).toBe(7);
	});

	test('applyOverrides pose les valeurs sur une copie', () => {
		const base = { wall_loops: 3, layer_height: 0.2 };
		const out = applyOverrides(base, { wall_loops: '5' });
		expect(out).toEqual({ wall_loops: 5, layer_height: 0.2 });
		expect(base.wall_loops).toBe(3);
	});
});

describe('buildProjectOverrides (sauvegarde, tranchage, validation)', () => {
	const resolved = { wall_loops: 3, layer_height: 0.2 };
	const input = (over: Partial<OverridesInput> = {}): OverridesInput => ({
		active: { printer: 'm', process: 'p', filaments: ['f1', 'f2'] },
		pending: emptyActivePresets(),
		process: { loadedId: 'p', resolved, current: { ...resolved, wall_loops: 5 } },
		printer: { loadedId: null, resolved: {}, current: {} },
		filament: { loadedId: null, slot: null, resolved: {}, current: {} },
		...over
	});

	test('brouillon chargé : seules les clés qui diffèrent du preset', () => {
		expect(buildProjectOverrides(input())).toEqual({ process: { wall_loops: 5 } });
	});

	test('rien de modifié → undefined', () => {
		expect(
			buildProjectOverrides(
				input({ process: { loadedId: 'p', resolved, current: { ...resolved } } })
			)
		).toBeUndefined();
	});

	test('preset pas encore chargé : surcharges enregistrées conservées', () => {
		const pending: ActivePresets = {
			printer: 'm',
			process: 'p',
			filaments: ['f1', 'f2'],
			overrides: { printer: { retraction_length: [1] }, filaments: [null, { x: 1 }] }
		};
		expect(buildProjectOverrides(input({ pending }))).toEqual({
			process: { wall_loops: 5 },
			printer: { retraction_length: [1] },
			filaments: [null, { x: 1 }]
		});
	});

	test('preset changé depuis (Abandonner / autre preset) : surcharge en attente ignorée', () => {
		const pending: ActivePresets = {
			printer: 'm0',
			process: 'p',
			filaments: ['fX'],
			overrides: { printer: { a: 1 }, filaments: [{ b: 2 }] }
		};
		expect(buildProjectOverrides(input({ pending }))).toEqual({ process: { wall_loops: 5 } });
	});

	test("brouillon d'un autre preset (changement en cours) : pas attribué au nouveau", () => {
		expect(
			buildProjectOverrides(
				input({ process: { loadedId: 'ancien', resolved, current: { wall_loops: 9 } } })
			)
		).toBeUndefined();
	});

	test('filament : brouillon du slot chargé + attente des autres slots', () => {
		const pending: ActivePresets = {
			...emptyActivePresets(),
			filaments: ['f1'],
			overrides: { filaments: [{ y: 1 }] }
		};
		const out = buildProjectOverrides(
			input({
				process: { loadedId: 'p', resolved, current: resolved },
				pending,
				filament: { loadedId: 'f2', slot: 1, resolved: { t: [200] }, current: { t: [215] } }
			})
		);
		expect(out).toEqual({ filaments: [{ y: 1 }, { t: [215] }] });
	});
});
