// Store de réglages (T043) : superposition preset/surcharges, marqueurs,
// reset par option (US2-AS6), validation à l'écriture, verrou.
import { describe, expect, it } from 'bun:test';

import { SettingsStore } from './store';

const effective = () => ({
	layer_height: 0.2,
	wall_loops: 3,
	bed_temperature_formula: 'by_first_filament'
});

describe('SettingsStore — superposition', () => {
	it('renvoie la valeur du preset tant qu’il n’y a pas de surcharge', () => {
		const s = new SettingsStore(effective());
		expect(s.value('layer_height')).toBe(0.2);
		expect(s.isModified('layer_height')).toBe(false);
	});

	it('une surcharge masque la valeur du preset et marque « modifié »', () => {
		const s = new SettingsStore(effective());
		expect(s.set('layer_height', 0.12).ok).toBe(true);
		expect(s.value('layer_height')).toBe(0.12);
		expect(s.presetValue('layer_height')).toBe(0.2);
		expect(s.isModified('layer_height')).toBe(true);
		expect(s.modifiedKeys()).toEqual(['layer_height']);
	});

	it('ne conserve pas une surcharge égale au preset', () => {
		const s = new SettingsStore(effective());
		s.set('layer_height', 0.2);
		expect(s.isModified('layer_height')).toBe(false);
		expect(s.modifiedKeys()).toEqual([]);
	});

	it('ne garde à la construction que les surcharges réellement différentes', () => {
		const s = new SettingsStore(effective(), { layer_height: 0.2, wall_loops: 5 });
		expect(s.isModified('layer_height')).toBe(false);
		expect(s.isModified('wall_loops')).toBe(true);
		expect(s.value('wall_loops')).toBe(5);
	});
});

describe('SettingsStore — reset (US2-AS6)', () => {
	it('reset ramène un paramètre à la valeur du preset', () => {
		const s = new SettingsStore(effective());
		s.set('wall_loops', 6);
		expect(s.isModified('wall_loops')).toBe(true);
		s.reset('wall_loops');
		expect(s.isModified('wall_loops')).toBe(false);
		expect(s.value('wall_loops')).toBe(3);
	});

	it('resetAll efface toutes les surcharges', () => {
		const s = new SettingsStore(effective());
		s.set('layer_height', 0.1);
		s.set('wall_loops', 4);
		s.resetAll();
		expect(s.modifiedKeys()).toEqual([]);
	});
});

describe('SettingsStore — validation', () => {
	it('rejette une écriture hors bornes sans modifier l’état', () => {
		const s = new SettingsStore(effective());
		const res = s.set('wall_loops', -5);
		expect(res.ok).toBe(false);
		expect(s.isModified('wall_loops')).toBe(false);
		expect(s.value('wall_loops')).toBe(3);
	});

	it('rejette une valeur d’enum inconnue', () => {
		const s = new SettingsStore(effective());
		expect(s.set('bed_temperature_formula', 'nope').ok).toBe(false);
		expect(s.set('bed_temperature_formula', 'by_highest_temp').ok).toBe(true);
	});

	it('rejette une clé inconnue du registre', () => {
		const s = new SettingsStore(effective());
		expect(s.set('clé_bidon', 1).ok).toBe(false);
	});
});

describe('SettingsStore — verrou', () => {
	it('empêche l’écriture d’un paramètre verrouillé', () => {
		const s = new SettingsStore(effective());
		s.setLocked('layer_height', true);
		expect(s.isLocked('layer_height')).toBe(true);
		expect(s.set('layer_height', 0.3).ok).toBe(false);
		expect(s.value('layer_height')).toBe(0.2);
		s.setLocked('layer_height', false);
		expect(s.set('layer_height', 0.3).ok).toBe(true);
	});
});

describe('SettingsStore — persistance', () => {
	it('overridesSnapshot ne contient que les diffs, en copie', () => {
		const s = new SettingsStore(effective());
		s.set('wall_loops', 7);
		const snap = s.overridesSnapshot();
		expect(snap).toEqual({ wall_loops: 7 });
		snap.wall_loops = 99; // la copie ne doit pas fuiter dans le store
		expect(s.value('wall_loops')).toBe(7);
	});
});
