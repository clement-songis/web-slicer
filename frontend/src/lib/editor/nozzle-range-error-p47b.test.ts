import { describe, expect, it } from 'vitest';
import { friendlySliceError } from './session';
import { isNozzleRangeError, nozzleRangeError } from './nozzle-range-error';

const ORCA =
	'Filament 1 is placed in the right nozzle, but the generated G-code path exceeds the printable range of the right nozzle.';

describe('erreur H2D « hors zone de la buse » (lot 47 H2D-a, phase 47b)', () => {
	it('restitue le texte d’Orca sans le préfixe du moteur', () => {
		expect(nozzleRangeError('tranchage impossible : ' + ORCA)).toBe(ORCA);
		expect(isNozzleRangeError(ORCA)).toBe(true);
	});

	it('sort de l’enveloppe du worker (E invalid_model)', () => {
		const raw =
			'worker sorti en code 1 — stderr :  — stdout : P 0.5 export E invalid_model tranchage impossible : ' +
			ORCA;
		expect(friendlySliceError(raw)).toBe(ORCA);
	});

	it('hauteur : plusieurs filaments, plusieurs buses, lien wiki', () => {
		const raw =
			'tranchage impossible : Filaments 1, 2 are placed in the left nozzle, but the generated G-code path exceeds the printable height of the left nozzle. Filament 3 is placed in the right nozzle, but the generated G-code path exceeds the printable height of the right nozzle. Open wiki for more information.';
		expect(nozzleRangeError(raw)?.split('\n')).toEqual([
			'Filaments 1, 2 are placed in the left nozzle, but the generated G-code path exceeds the printable height of the left nozzle.',
			'Filament 3 is placed in the right nozzle, but the generated G-code path exceeds the printable height of the right nozzle.',
			'Open wiki for more information.'
		]);
	});

	it('imprimante non Bambu : « Tool n »', () => {
		const raw =
			'tranchage impossible : Filament 2 is placed in the Tool 2, but the generated G-code path exceeds the printable range of the Tool 2.';
		expect(nozzleRangeError(raw)).toBe(
			'Filament 2 is placed in the Tool 2, but the generated G-code path exceeds the printable range of the Tool 2.'
		);
	});

	it('autres refus : inchangés', () => {
		expect(
			nozzleRangeError('tranchage impossible : One object has an empty first layer')
		).toBeNull();
		expect(isNozzleRangeError('Slicing failed')).toBe(false);
		expect(isNozzleRangeError(null)).toBe(false);
	});
});
