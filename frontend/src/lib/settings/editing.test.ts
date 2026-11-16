import { describe, expect, it } from 'vitest';
import { defaultableKeys, orcaDefault, withOrcaDefaults } from './editing';

import {
	changedPresetValues,
	clonePresetValues,
	hasPresetChanges,
	presetValuesForSave,
	settingEqual
} from './editing';

describe('édition d’un preset résolu', () => {
	it('ne marque pas une simple différence de format numérique', () => {
		expect(settingEqual(20, '20')).toBe(true);
		expect(settingEqual('50%', '50%')).toBe(true);
		expect(settingEqual('50%', '50')).toBe(false);
		expect(settingEqual([0.4, 0], ['0.4', 0])).toBe(true);
	});

	it('ignore les valeurs neutres posées par les widgets sur une clé absente de /resolved', () => {
		const resolved = { layer_height: 0.2 };
		const current = {
			layer_height: 0.2,
			bed_mesh_probe_distance: [0, 0],
			disable_m73: false,
			pellet_modded_printer: true
		};
		expect(changedPresetValues(resolved, current)).toEqual({ pellet_modded_printer: true });
		// Surcharge nullable explicitement posée, même au premier choix : conservée.
		expect(changedPresetValues({}, { filament_retract_lift_enforce: ['All Surfaces'] })).toEqual({
			filament_retract_lift_enforce: ['All Surfaces']
		});
		// Clé présente dans /resolved : un retour à 0/false reste une vraie modification.
		expect(changedPresetValues({ disable_m73: true }, { disable_m73: false })).toEqual({
			disable_m73: false
		});
	});

	it('compare une clé absente de /resolved au défaut Orca du registre', () => {
		// Égale au défaut Orca : valeur effective inchangée, pas d'astérisque (parité Orca).
		expect(changedPresetValues({}, { wall_direction: 'ccw' })).toEqual({});
		// Différente du défaut : vraie modification, conservée.
		expect(changedPresetValues({}, { wall_direction: 'cw' })).toEqual({ wall_direction: 'cw' });
		expect(changedPresetValues({}, { enable_power_loss_recovery: 'enable' })).toEqual({
			enable_power_loss_recovery: 'enable'
		});
		// Valeur neutre du widget point, différente du défaut 50x50 : toujours ignorée.
		expect(changedPresetValues({}, { bed_mesh_probe_distance: [0, 0] })).toEqual({});
	});

	it('15 ≡ « 15% » pour une option pourcentage, pas ailleurs', () => {
		expect(settingEqual(15, '15%', true)).toBe(true);
		expect(settingEqual('15%', 15, true)).toBe(true);
		expect(settingEqual(15, '16%', true)).toBe(false);
		expect(settingEqual(15, '15%')).toBe(false);
		expect(hasPresetChanges({ sparse_infill_density: 15 }, { sparse_infill_density: '15%' })).toBe(
			false
		);
		expect(
			changedPresetValues({ sparse_infill_density: 15 }, { sparse_infill_density: '20%' })
		).toEqual({
			sparse_infill_density: '20%'
		});
	});

	it('ne retourne que les valeurs réellement modifiées', () => {
		const resolved = { wall_loops: 2, layer_height: 0.2 };
		const current = { wall_loops: 5, layer_height: '0.2' };
		expect(hasPresetChanges(resolved, current)).toBe(true);
		expect(changedPresetValues(resolved, current)).toEqual({ wall_loops: 5 });
		expect(hasPresetChanges(resolved, resolved)).toBe(false);
	});

	it('dérive un système avec les seules différences', () => {
		expect(
			presetValuesForSave(
				{ wall_loops: 2, layer_height: 0.2 },
				{ wall_loops: 5, layer_height: 0.2 },
				{ wall_loops: 2, compatible_prints: ['base'] },
				true
			)
		).toEqual({ wall_loops: 5 });
	});

	it('met à jour un utilisateur sans perdre ses autres surcharges', () => {
		expect(
			presetValuesForSave(
				{ wall_loops: 3, layer_height: 0.16 },
				{ wall_loops: 5, layer_height: 0.16 },
				{ layer_height: 0.16 },
				false
			)
		).toEqual({ layer_height: 0.16, wall_loops: 5 });
	});

	it('clone les tableaux pour isoler le brouillon de la base', () => {
		const source = { nozzle_diameter: [0.4] };
		const copy = clonePresetValues(source);
		(copy.nozzle_diameter as number[])[0] = 0.6;
		expect(source.nozzle_diameter).toEqual([0.4]);
	});
});

describe('défauts Orca des clés absentes', () => {
	it('precise_outer_wall absent → vrai (PrintConfig.cpp : ConfigOptionBool{true})', () => {
		expect(withOrcaDefaults('process', { wall_loops: 3 }).precise_outer_wall).toBe(true);
	});
	it('les clés présentes ne changent pas', () => {
		expect(withOrcaDefaults('process', { precise_outer_wall: false }).precise_outer_wall).toBe(
			false
		);
	});
	it('formats : vecteur, pourcentage, flottant-ou-%, enum, points', () => {
		expect(orcaDefault('retract_before_wipe')).toEqual([100]);
		expect(typeof orcaDefault('sparse_infill_density')).toBe('number');
		expect(orcaDefault('z_hop_types')).toEqual(['Slope Lift']);
		expect(orcaDefault('printable_area')).toEqual(['0x0', '200x0', '200x200', '0x200']);
		expect(orcaDefault('filament_ironing_flow')).toBeUndefined();
	});
	it('chaque type a des clés couvertes', () => {
		expect(defaultableKeys('process').length).toBeGreaterThan(200);
	});
});
