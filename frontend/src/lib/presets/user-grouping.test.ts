import { describe, expect, test } from 'vitest';
import {
	clearUserPresetMeta,
	groupUserPresets,
	loadUserPresetMeta,
	metaFromValues,
	UNSPECIFIED
} from './user-grouping';

const user = [
	{ id: 'u1', name: 'Mon PETG' },
	{ id: 'u2', name: 'Mon PLA soie' },
	{ id: 'u3', name: 'Inconnu' },
	{ id: 'u4', name: 'Mon PLA mat' }
];
const meta = {
	u1: { type: 'PETG', vendor: 'Polymaker' },
	u2: { type: 'PLA', vendor: 'Sunlu' },
	u4: { type: 'PLA', vendor: 'Polymaker' }
};

describe('« Grouper les presets filament utilisateur » (Orca group_filament_presets)', () => {
	test('Tous : un groupe ; Aucun : sans groupe (libellé vide)', () => {
		expect(groupUserPresets(user, 'all', meta)).toEqual([{ label: 'Mes presets', presets: user }]);
		expect(groupUserPresets(user, 'none', meta)).toEqual([{ label: '', presets: user }]);
		expect(groupUserPresets([], 'type', meta)).toEqual([]);
	});

	test('Par type / par fabricant : groupes triés, « Non précisé » en dernier', () => {
		expect(
			groupUserPresets(user, 'type', meta).map((g) => [g.label, g.presets.map((p) => p.id)])
		).toEqual([
			['Mes presets · PETG', ['u1']],
			['Mes presets · PLA', ['u2', 'u4']],
			[`Mes presets · ${UNSPECIFIED}`, ['u3']]
		]);
		expect(groupUserPresets(user, 'vendor', meta).map((g) => g.label)).toEqual([
			'Mes presets · Polymaker',
			'Mes presets · Sunlu',
			`Mes presets · ${UNSPECIFIED}`
		]);
	});

	test('valeurs résolues : premier élément des vecteurs ; chargement mis en cache, échec isolé', async () => {
		expect(metaFromValues({ filament_type: ['PLA'], filament_vendor: ['  Bambu '] })).toEqual({
			type: 'PLA',
			vendor: 'Bambu'
		});
		expect(metaFromValues(null)).toEqual({ type: undefined, vendor: undefined });
		clearUserPresetMeta();
		let calls = 0;
		const fetcher = async (id: string) => {
			calls++;
			if (id === 'bad') throw new Error('500');
			return { filament_type: ['ABS'] };
		};
		const first = await loadUserPresetMeta(['a', 'bad'], fetcher);
		expect(first).toEqual({ a: { type: 'ABS', vendor: undefined }, bad: {} });
		await loadUserPresetMeta(['a'], fetcher);
		expect(calls).toBe(2);
	});
});
