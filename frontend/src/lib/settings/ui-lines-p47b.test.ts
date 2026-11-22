import { describe, expect, it } from 'vitest';
import { PARAMS } from '../../generated/params';
import { LINE_OPTION_LABELS, sectionLines, sectionRows } from './ui-lines';

const labelOf = (key: string) => PARAMS[key]?.label;

describe('ui-lines (lignes composées d’Orca, phase 47b)', () => {
	it('groupe la température de buse sous « Nozzle »', () => {
		const lines = sectionLines('filament', 'Filament', 'Print temperature');
		const rows = sectionRows(
			['nozzle_temperature_initial_layer', 'nozzle_temperature'],
			lines,
			labelOf
		);
		expect(rows).toEqual([
			{
				kind: 'line',
				line: lines[0],
				options: ['nozzle_temperature_initial_layer', 'nozzle_temperature']
			}
		]);
		expect(lines[0].label).toBe('Nozzle');
	});

	it('une ligne par type de plaque, dans l’ordre d’Orca', () => {
		const lines = sectionLines('filament', 'Filament', 'Bed temperature');
		const keys = lines.flatMap((l) => l.options);
		const rows = sectionRows(keys, lines, labelOf);
		expect(rows.map((r) => (r.kind === 'line' ? r.line.label : '?'))).toEqual([
			'Cool Plate (SuperTack)',
			'Cool Plate',
			'Textured Cool Plate',
			'Engineering Plate',
			'Smooth PEI Plate / High Temp Plate',
			'Textured PEI Plate'
		]);
		for (const r of rows) if (r.kind === 'line') expect(r.options).toHaveLength(2);
	});

	it('chambre : option seule (case) puis ligne Target / Minimal', () => {
		const lines = sectionLines('filament', 'Filament', 'Print chamber temperature');
		const rows = sectionRows(
			['activate_chamber_temp_control', 'chamber_temperature', 'chamber_minimal_temperature'],
			lines,
			labelOf
		);
		expect(rows[0]).toEqual({ kind: 'option', option: 'activate_chamber_temp_control', index: 0 });
		expect(rows[1]).toMatchObject({
			kind: 'line',
			options: ['chamber_temperature', 'chamber_minimal_temperature']
		});
		expect(LINE_OPTION_LABELS.chamber_temperature).toBe('Target');
	});

	it('garde seulement les options visibles (mode / recherche)', () => {
		const lines = sectionLines('filament', 'Filament', 'Print temperature');
		const rows = sectionRows(['nozzle_temperature'], lines, labelOf);
		expect(rows).toMatchObject([{ kind: 'line', options: ['nozzle_temperature'] }]);
	});

	it('ligne à option unique de même libellé : pas d’en-tête', () => {
		const lines = sectionLines('machine', 'Motion ability', 'Speed limitation');
		const rows = sectionRows(['machine_max_speed_x', 'machine_max_speed_y'], lines, labelOf);
		expect(rows.every((r) => r.kind === 'option')).toBe(true);
	});

	it('section sans ligne : options inchangées', () => {
		expect(sectionRows(['a', { dynamic: 'x' }], [], labelOf)).toEqual([
			{ kind: 'option', option: 'a', index: 0 },
			{ kind: 'option', option: { dynamic: 'x' }, index: 1 }
		]);
	});
});
