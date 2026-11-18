// Phase 27 — afficher l'onglet « Forçage des réglages » ne doit pas créer de
// surcharge filament : [null] / [null, null] (« nil ») ne sont pas des saisies.
import { describe, expect, it } from 'vitest';
import { changedPresetValues, isWidgetFill } from './editing';
import { isOverrideActive } from './special/groups';

describe('surcharges filament « nil »', () => {
	it('un vecteur entièrement nul est un remplissage de widget', () => {
		expect(isWidgetFill('filament_retract_lift_above', [null])).toBe(true);
		expect(isWidgetFill('filament_retract_lift_above', [null, null])).toBe(true);
		expect(isWidgetFill('filament_retract_lift_above', [0.2])).toBe(false);
		expect(isWidgetFill('filament_wipe', [false])).toBe(false);
	});

	it('le brouillon ignore une clé absente devenue [null]', () => {
		expect(changedPresetValues({}, { filament_retract_lift_above: [null] })).toEqual({});
		expect(
			changedPresetValues(
				{ filament_wipe: [null, null] },
				{ filament_wipe: [null, null], filament_retract_lift_enforce: [null] }
			)
		).toEqual({});
	});

	it('une vraie surcharge reste un changement', () => {
		expect(changedPresetValues({ filament_wipe: [null, null] }, { filament_wipe: [true] })).toEqual(
			{
				filament_wipe: [true]
			}
		);
	});

	it('isOverrideActive : [null, null] = N/A', () => {
		expect(isOverrideActive([null, null])).toBe(false);
		expect(isOverrideActive([false, null])).toBe(true);
		expect(isOverrideActive(null)).toBe(false);
		expect(isOverrideActive(0)).toBe(true);
	});
});
