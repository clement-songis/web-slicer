import { describe, expect, test } from 'vitest';
import { nozzleFlowChoices, setNozzleFlow } from './nozzle-flow';

describe('type de débit de buse par extrudeur (ExtruderGroup d’Orca)', () => {
	test('absent sans variante « High Flow »', () => {
		expect(nozzleFlowChoices({ nozzle_diameter: ['0.4'] })).toEqual([]);
		expect(
			nozzleFlowChoices({
				extruder_variant_list: ['Direct Drive Standard'],
				nozzle_diameter: ['0.4']
			})
		).toEqual([]);
	});

	test('un sélecteur par extrudeur, valeur lue de nozzle_volume_type', () => {
		const machine = {
			extruder_variant_list: [
				'Direct Drive Standard,Direct Drive High Flow',
				'Direct Drive Standard,Direct Drive High Flow'
			],
			nozzle_diameter: ['0.4', '0.4'],
			nozzle_volume_type: ['Standard', 'High Flow']
		};
		expect(nozzleFlowChoices(machine)).toEqual([
			{ index: 0, options: ['Standard', 'High Flow'], value: 'Standard' },
			{ index: 1, options: ['Standard', 'High Flow'], value: 'High Flow' }
		]);
		expect(setNozzleFlow(machine, 0, 'High Flow')).toEqual(['High Flow', 'High Flow']);
	});

	test('vecteur absent : Standard par défaut, complété à la taille des extrudeurs', () => {
		const machine = {
			extruder_variant_list: ['Direct Drive Standard,Direct Drive High Flow'],
			nozzle_diameter: ['0.4']
		};
		expect(nozzleFlowChoices(machine)[0].value).toBe('Standard');
		expect(setNozzleFlow(machine, 0, 'High Flow')).toEqual(['High Flow']);
	});
});
