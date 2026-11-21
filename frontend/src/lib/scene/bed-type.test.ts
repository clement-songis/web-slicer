import { describe, expect, it } from 'vitest';
import {
	bedTypeToPlateType,
	plateTypeToBedType,
	printerDefaultBedType,
	printerDefaultPlateType
} from './bed-type';
import { PLATE_TYPES } from '../settings/special/dialogs';

describe('printerDefaultBedType (Orca Preset::get_default_bed_type)', () => {
	it('imprimante générique (Voron, P1S…) sans default_bed_type → plaque haute température', () => {
		expect(printerDefaultBedType({ printer_model: 'Voron 2.4 350' })).toBe('High Temp Plate');
		expect(printerDefaultBedType({ printer_model: 'Bambu Lab P1S', default_bed_type: '' })).toBe(
			'High Temp Plate'
		);
		expect(printerDefaultBedType({})).toBe('High Temp Plate');
	});

	it('X1 / X1 Carbon / X1E (BL-P001, BL-P002, C13) → plaque froide', () => {
		for (const model of ['Bambu Lab X1 Carbon', 'Bambu Lab X1', 'Bambu Lab X1E'])
			expect(printerDefaultBedType({ printer_model: model })).toBe('Cool Plate');
		expect(printerDefaultBedType({ printer_model: ['Bambu Lab X1'] })).toBe('Cool Plate');
	});

	it('default_bed_type numérique → BedType(n)', () => {
		expect(printerDefaultBedType({ default_bed_type: '4' })).toBe('Textured PEI Plate');
		expect(printerDefaultBedType({ default_bed_type: '1', printer_model: 'Voron' })).toBe(
			'Cool Plate'
		);
		expect(printerDefaultBedType({ default_bed_type: '6' })).toBe('Supertack Plate');
		// Prioritaire sur le modèle.
		expect(printerDefaultBedType({ default_bed_type: '3', printer_model: 'Bambu Lab X1' })).toBe(
			'High Temp Plate'
		);
	});

	it('default_bed_type textuel ou invalide → btPEI (Orca ne lit que la forme numérique)', () => {
		expect(printerDefaultBedType({ default_bed_type: 'Textured PEI Plate' })).toBe(
			'High Temp Plate'
		);
		expect(printerDefaultBedType({ default_bed_type: '0', printer_model: 'Bambu Lab X1' })).toBe(
			'High Temp Plate'
		);
		expect(printerDefaultBedType({ default_bed_type: '-2' })).toBe('High Temp Plate');
	});

	it('BedType hors du sélecteur → 1re entrée (Cool Plate)', () => {
		expect(printerDefaultBedType({ default_bed_type: '9' })).toBe('Cool Plate');
	});
});

describe('libellés WebSlicer ↔ clés Orca', () => {
	it('aller-retour sur tous les PLATE_TYPES', () => {
		for (const pt of PLATE_TYPES) {
			const bed = plateTypeToBedType(pt.label);
			expect(bed).not.toBeNull();
			expect(bedTypeToPlateType(bed!)).toBe(pt.label);
		}
		expect(bedTypeToPlateType('SuperTack Plate')).toBe('Supertack Plate');
		expect(bedTypeToPlateType('Default Plate')).toBeNull();
		expect(plateTypeToBedType('High Temp Plate')).toBe('High Temp Plate');
	});

	it('printerDefaultPlateType donne un libellé du sélecteur', () => {
		expect(printerDefaultPlateType({ printer_model: 'Voron 2.4 350' })).toBe(
			'Smooth High Temp Plate'
		);
		expect(printerDefaultPlateType({ printer_model: 'Bambu Lab X1 Carbon' })).toBe(
			'Cool Plate / PLA Plate'
		);
		expect(printerDefaultPlateType({ default_bed_type: '4' })).toBe('Textured PEI Plate');
	});
});
