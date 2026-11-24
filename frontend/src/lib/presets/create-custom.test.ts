import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { setLocale } from '#lib/i18n';

beforeAll(() => setLocale('fr'));
afterAll(() => setLocale('en'));
import {
	filamentPresetName,
	filamentValues,
	printerPresetName,
	printerValues,
	validateFilament,
	validatePrinter
} from './create-custom';

describe('presets personnalisés', () => {
	it('filament : nom Orca, validation, valeurs', () => {
		const f = { vendor: ' Atelier/3D ', type: 'PETG', serial: 'Mat' };
		expect(filamentPresetName(f)).toBe('Atelier3D PETG Mat');
		expect(validateFilament(f, [])).toBeNull();
		expect(validateFilament({ ...f, serial: ' ' }, [])).toMatch(/série/);
		expect(validateFilament(f, ['Atelier3D PETG Mat'])).toMatch(/existe/);
		expect(filamentValues(f)).toEqual({ filament_vendor: ['Atelier3D'], filament_type: ['PETG'] });
	});
	it('imprimante : nom, contour du plateau, origine centrée', () => {
		const p = {
			vendor: 'Atelier',
			model: 'Cube 300',
			nozzle: 0.6,
			width: 300,
			depth: 250,
			height: 280
		};
		expect(printerPresetName(p)).toBe('Atelier Cube 300 0.6 nozzle');
		expect(printerValues(p).printable_area).toEqual(['0x0', '300x0', '300x250', '0x250']);
		expect(printerValues({ ...p, centered: true }).printable_area).toEqual([
			'-150x-125',
			'150x-125',
			'150x125',
			'-150x125'
		]);
		expect(validatePrinter({ ...p, height: 0 }, [])).toMatch(/hauteur/);
		expect(validatePrinter(p, [])).toBeNull();
	});
});
