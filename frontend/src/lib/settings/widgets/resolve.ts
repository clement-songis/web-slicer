// Résolution type de registre → widget d'édition (T041). Fonction pure,
// testée isolément (aucun import Svelte) : la table kind→composant vit dans
// `registry.ts`. La règle miroir d'OrcaSlicer : le type `ConfigOptionType`
// choisit le contrôle, quelques cas particuliers (couleur, G-code multiligne,
// enums ouverts) s'appuient sur `gui_type` / la clé.

import type { ParamDef } from '../../../generated/params';

/** Contrôle d'édition associé à un paramètre. */
export type WidgetKind =
	| 'bool'
	| 'int'
	| 'float'
	| 'percent'
	| 'floatOrPercent'
	| 'enum'
	| 'string'
	| 'strings'
	| 'multiline'
	| 'point'
	| 'color';

/** Les champs G-code personnalisés sont édités en zone multiligne (parité). */
export function isGcodeParam(key: string): boolean {
	return key.includes('gcode');
}

/**
 * Choisit le widget d'un paramètre. Ordre de priorité : couleur (gui_type),
 * enum (type énuméré ou liste de valeurs ouverte), G-code multiligne, puis le
 * type de base (suffixes `s`/`Nullable` regroupés sur le même contrôle).
 */
export function resolveWidgetKind(def: ParamDef): WidgetKind {
	if (def.guiType === 'color') return 'color';

	// Enums fermés (coEnum*) et enums « ouverts » (numérique/texte + valeurs
	// suggérées, gui_type *_enum_open / select_open) partagent le sélecteur.
	if (def.type.startsWith('coEnum') || def.enumValues.length > 0) return 'enum';

	if ((def.type === 'coString' || def.type === 'coStrings') && isGcodeParam(def.key)) {
		return 'multiline';
	}

	switch (def.type) {
		case 'coBool':
		case 'coBools':
		case 'coBoolsNullable':
			return 'bool';
		case 'coInt':
		case 'coInts':
			return 'int';
		case 'coFloat':
		case 'coFloats':
		case 'coFloatsNullable':
			return 'float';
		case 'coPercent':
		case 'coPercents':
		case 'coPercentsNullable':
			return 'percent';
		case 'coFloatOrPercent':
		case 'coFloatsOrPercents':
			return 'floatOrPercent';
		case 'coPoint':
		case 'coPoints':
		case 'coPointsGroups':
			return 'point';
		case 'coString':
			return 'string';
		case 'coStrings':
			return 'strings';
		default:
			return 'string';
	}
}
