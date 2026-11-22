// Phase 46 (audit 44 DEV1-a) : préréglage par défaut de « Déclarer
// manuellement » et avertissement d'envoi vers une imprimante d'un autre profil.
import { describe, expect, it } from 'vitest';
import { defaultDeclarePreset, presetMismatch } from './send';

const machines = [
	{ id: 'afinia', name: 'Afinia H+1(HS) 0.4 nozzle' },
	{ id: 'voron', name: 'Voron 2.4 350 0.4 nozzle' },
	{ id: 'voron-user', name: 'Voron Atelier', inherits: 'Voron 2.4 350 0.4 nozzle' },
	{ id: 'p1s', name: 'Bambu Lab P1S 0.4 nozzle' }
];

describe('defaultDeclarePreset', () => {
	it('preset du projet d’abord', () => {
		expect(defaultDeclarePreset(machines, 'p1s', [{ machine_preset_id: 'voron' }])).toBe('p1s');
	});
	it('sinon la dernière imprimante déclarée, jamais le premier du catalogue', () => {
		expect(
			defaultDeclarePreset(machines, null, [
				{ machine_preset_id: 'p1s' },
				{ machine_preset_id: 'voron' }
			])
		).toBe('voron');
		expect(defaultDeclarePreset(machines, 'inconnu', [{ machine_preset_id: 'gone' }])).toBe('');
		expect(defaultDeclarePreset(machines, null, [])).toBe('');
	});
});

describe('presetMismatch', () => {
	it('même preset ou preset dérivé : pas d’avertissement', () => {
		expect(presetMismatch(machines[1], { machine_preset_id: 'voron' }, machines)).toBeNull();
		expect(presetMismatch(machines[1], { machine_preset_id: 'voron-user' }, machines)).toBeNull();
		expect(presetMismatch(machines[2], { machine_preset_id: 'voron' }, machines)).toBeNull();
	});
	it('autre imprimante : noms des deux profils', () => {
		expect(presetMismatch(machines[1], { machine_preset_id: 'afinia' }, machines)).toEqual({
			sliced: 'Voron 2.4 350 0.4 nozzle',
			target: 'Afinia H+1(HS) 0.4 nozzle'
		});
	});
	it('printer_model connu : seule règle (comme le 409 serveur)', () => {
		const v06 = { id: 'v06', name: 'Voron 2.4 350 0.6 nozzle' };
		const list = [...machines, v06];
		expect(
			presetMismatch(machines[1], { machine_preset_id: 'v06' }, list, {
				sliced: 'Voron 2.4 350',
				target: 'Voron 2.4 350'
			})
		).toBeNull();
		expect(
			presetMismatch(machines[1], { machine_preset_id: 'p1s' }, list, {
				sliced: 'Voron 2.4 350',
				target: 'Bambu Lab P1S'
			})
		).toEqual({ sliced: 'Voron 2.4 350 0.4 nozzle', target: 'Bambu Lab P1S 0.4 nozzle' });
		// Modèle connu d'un seul côté (imprimante personnalisée) : autorisé.
		expect(
			presetMismatch(machines[1], { machine_preset_id: 'afinia' }, list, {
				sliced: 'Voron 2.4 350',
				target: ''
			})
		).toBeNull();
	});
	it('preset inconnu ou absent : rien', () => {
		expect(presetMismatch(null, { machine_preset_id: 'afinia' }, machines)).toBeNull();
		expect(presetMismatch(machines[1], { machine_preset_id: 'x' }, machines)).toBeNull();
	});
});
