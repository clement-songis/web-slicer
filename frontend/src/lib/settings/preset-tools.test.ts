import { describe, expect, it } from 'vitest';
import {
	diffPresets,
	filterPlaceholders,
	formatSetting,
	insertAt,
	placeholderGroups,
	transferChanges
} from './preset-tools';

describe('outils de presets', () => {
	it('diff : clés des onglets du type, ordre des onglets', () => {
		const d = diffPresets(
			'process',
			{ wall_loops: 2, layer_height: 0.2 },
			{ wall_loops: '3', layer_height: '0.2' }
		);
		expect(d.map((x) => x.key)).toEqual(['wall_loops']);
		expect(d[0].page).toBeTruthy();
	});
	it('format court des valeurs', () => {
		expect(formatSetting(['a', 'b'])).toBe('a, b');
		expect(formatSetting('G28\nG1 X0')).toBe('G28 (… 2 lignes)');
		expect(formatSetting(undefined)).toBe('—');
	});
	it('transfert : modifications posées sur le preset cible', () => {
		expect(
			transferChanges({ wall_loops: 4 }, { wall_loops: 2, sparse_infill_density: '15%' })
		).toEqual({
			wall_loops: 4,
			sparse_infill_density: '15%'
		});
	});
	it('variables du G-code : spécifiques, états, presets ; vecteurs entre crochets', () => {
		const groups = placeholderGroups('layer_change_gcode');
		expect(groups[0].items.map((i) => i.key)).toContain('layer_z');
		const all = groups.flatMap((g) => g.items);
		expect(all.find((i) => i.key === 'layer_num')?.insert).toBe('{layer_num}');
		expect(all.find((i) => i.key === 'filament_diameter')?.insert).toBe(
			'{filament_diameter[current_extruder]}'
		);
		expect(all.find((i) => i.key === 'position')?.insert).toBe('{position[]}');
		expect(
			filterPlaceholders(groups, 'hauteur', (s) =>
				s === 'Layer Z' ? 'Hauteur de couche' : s
			).flatMap((g) => g.items.map((i) => i.key))
		).toContain('layer_z');
	});
	it('insertion au curseur, curseur entre crochets vides', () => {
		expect(insertAt('G28\n', 4, 4, '{layer_z}')).toEqual({ value: 'G28\n{layer_z}', cursor: 13 });
		expect(insertAt('', 0, 0, '{position[]}').cursor).toBe(10);
	});
});
