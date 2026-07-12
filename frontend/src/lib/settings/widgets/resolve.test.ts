// Mapping type de registre → widget (T041). Vérifie chaque cas de base, les
// cas particuliers (couleur, G-code multiligne, enums ouverts), et l'exhaustivité
// sur les 858 paramètres réels (aucun ne tombe dans un trou de résolution).
import { describe, expect, it } from 'vitest';

import { PARAMS, type ParamDef } from '../../../generated/params';
import { resolveWidgetKind, isGcodeParam, type WidgetKind } from './resolve';

function def(partial: Partial<ParamDef> & Pick<ParamDef, 'type'>): ParamDef {
	return {
		key: 'k',
		mode: 'advanced',
		label: 'K',
		tooltip: '',
		sidetext: '',
		category: '',
		group: '',
		nullable: false,
		min: null,
		max: null,
		enumValues: [],
		enumLabels: [],
		guiType: '',
		ratioOver: '',
		default: null,
		...partial
	};
}

describe('resolveWidgetKind', () => {
	it('associe chaque type de base à son widget', () => {
		const cases: Array<[ParamDef['type'], WidgetKind]> = [
			['coBool', 'bool'],
			['coBools', 'bool'],
			['coBoolsNullable', 'bool'],
			['coInt', 'int'],
			['coInts', 'int'],
			['coFloat', 'float'],
			['coFloats', 'float'],
			['coFloatsNullable', 'float'],
			['coPercent', 'percent'],
			['coPercents', 'percent'],
			['coPercentsNullable', 'percent'],
			['coFloatOrPercent', 'floatOrPercent'],
			['coFloatsOrPercents', 'floatOrPercent'],
			['coPoint', 'point'],
			['coPoints', 'point'],
			['coPointsGroups', 'point'],
			['coString', 'string'],
			['coStrings', 'strings']
		];
		for (const [type, kind] of cases) {
			expect(resolveWidgetKind(def({ type }))).toBe(kind);
		}
	});

	it('résout les enums fermés et ouverts vers le sélecteur', () => {
		expect(resolveWidgetKind(def({ type: 'coEnum' }))).toBe('enum');
		expect(resolveWidgetKind(def({ type: 'coEnumsNullable' }))).toBe('enum');
		// Enum ouvert : type numérique + valeurs suggérées.
		expect(resolveWidgetKind(def({ type: 'coInt', enumValues: ['0', '1'] }))).toBe('enum');
	});

	it('détecte la couleur via gui_type', () => {
		expect(resolveWidgetKind(def({ type: 'coString', guiType: 'color' }))).toBe('color');
		expect(resolveWidgetKind(def({ type: 'coStrings', guiType: 'color' }))).toBe('color');
	});

	it('édite les G-code personnalisés en multiligne', () => {
		expect(resolveWidgetKind(def({ type: 'coString', key: 'machine_start_gcode' }))).toBe(
			'multiline'
		);
		expect(resolveWidgetKind(def({ type: 'coStrings', key: 'filament_start_gcode' }))).toBe(
			'multiline'
		);
		// Une chaîne ordinaire reste un champ simple.
		expect(resolveWidgetKind(def({ type: 'coString', key: 'printer_notes' }))).toBe('string');
	});

	it('donne la priorité couleur/enum sur le G-code', () => {
		// gcode_flavor est un enum : sélecteur malgré « gcode » dans la clé.
		const flavor = PARAMS['gcode_flavor'];
		expect(flavor.type.startsWith('coEnum')).toBe(true);
		expect(resolveWidgetKind(flavor)).toBe('enum');
	});

	it('résout les 4 couleurs réelles du registre', () => {
		for (const key of [
			'filament_colour',
			'extruder_colour',
			'default_filament_colour',
			'material_colour'
		]) {
			expect(resolveWidgetKind(PARAMS[key])).toBe('color');
		}
	});

	it('résout les 858 paramètres sans trou', () => {
		const known = new Set<WidgetKind>([
			'bool',
			'int',
			'float',
			'percent',
			'floatOrPercent',
			'enum',
			'string',
			'strings',
			'multiline',
			'point',
			'color'
		]);
		for (const p of Object.values(PARAMS)) {
			expect(known.has(resolveWidgetKind(p))).toBe(true);
		}
	});
});

describe('isGcodeParam', () => {
	it('reconnaît les clés de G-code', () => {
		expect(isGcodeParam('machine_start_gcode')).toBe(true);
		expect(isGcodeParam('layer_change_gcode')).toBe(true);
		expect(isGcodeParam('layer_height')).toBe(false);
	});
});
