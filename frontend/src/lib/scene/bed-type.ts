// Type de plaque par défaut de l'imprimante (phase 44b) — parité Orca 2.4.1.
//
// Orca n'initialise pas le type de plaque d'un projet à « Cool Plate » (défaut
// de l'option `curr_bed_type`) : à la sélection de l'imprimante,
// `Sidebar::update_presets` (Plater.cpp:2513-2540) choisit
// `Preset::get_default_bed_type` (Preset.cpp:966) et les plateaux « par
// défaut » le suivent (`PartPlate::get_bed_type`). Côté WebSlicer, chaque
// plateau porte son libellé `plateType` (sélecteur « Type de plaque »), que le
// serveur traduit en `curr_bed_type` (lot 44, B-1) : un nouveau plateau prend
// donc le type par défaut de l'imprimante au lieu de « Cool Plate ».
import { PLATE_TYPES } from '../settings/special/dialogs';

/** `BedType` d'Orca (PrintConfig.hpp:314), btPC = 1 … btSuperTack = 6 → clé `s_keys_map_BedType`. */
export const ORCA_BED_TYPES = [
	'Default Plate',
	'Cool Plate',
	'Engineering Plate',
	'High Temp Plate',
	'Textured PEI Plate',
	'Textured Cool Plate',
	'Supertack Plate'
] as const;

/** Libellé WebSlicer (`PLATE_TYPES`) ↔ clé Orca (même table que backend/src/models/threemf.rs). */
const PLATE_TYPE_BED_TYPES: ReadonlyArray<readonly [string, string]> = [
	['Cool Plate / PLA Plate', 'Cool Plate'],
	['Supertack Plate', 'Supertack Plate'],
	['Engineering Plate', 'Engineering Plate'],
	['Smooth High Temp Plate', 'High Temp Plate'],
	['Textured Cool Plate', 'Textured Cool Plate'],
	['Textured PEI Plate', 'Textured PEI Plate']
];

/**
 * Modèles dont `get_printer_type` (model_id du modèle de vendeur nommé par
 * `printer_model`) vaut « BL-P001 », « BL-P002 » ou « C13 » : Orca leur donne
 * la plaque froide (btPC). Noms des modèles de resources/profiles/BBL.json.
 */
const COOL_PLATE_MODELS = new Set(['Bambu Lab X1 Carbon', 'Bambu Lab X1', 'Bambu Lab X1E']);

function first(v: unknown): string {
	if (Array.isArray(v)) return v.length ? String(v[0] ?? '') : '';
	return v === undefined || v === null ? '' : String(v);
}

/** `atoi` de la libc : entier en tête de chaîne (espaces et signe admis), 0 sinon. */
function atoi(text: string): number {
	const m = /^\s*([+-]?\d+)/.exec(text);
	return m ? Number.parseInt(m[1], 10) : 0;
}

/**
 * `Preset::get_default_bed_type` (Orca 2.4.1, Preset.cpp:966) → valeur Orca de
 * `curr_bed_type`, depuis les valeurs du preset machine.
 * - `default_bed_type` non vide : entier > 0 → `BedType(n)` ; sinon btPEI (Orca
 *   n'accepte que la forme numérique malgré l'infobulle : « Textured PEI Plate »
 *   en texte donne aussi btPEI, avec une erreur au journal).
 * - sinon X1 / X1 Carbon / X1E → btPC, toute autre imprimante → btPEI.
 * Une valeur hors de la liste du sélecteur (n ≥ btCount) est remplacée par
 * sa 1re entrée (`set_bed_type_accord_combox` → index 0 : « Cool Plate »).
 */
export function printerDefaultBedType(machine: Record<string, unknown>): string {
	const configured = first(machine.default_bed_type);
	if (configured !== '') {
		const n = atoi(configured);
		if (n > 0) return ORCA_BED_TYPES[n] ?? ORCA_BED_TYPES[1];
		return 'High Temp Plate';
	}
	return COOL_PLATE_MODELS.has(first(machine.printer_model)) ? 'Cool Plate' : 'High Temp Plate';
}

/** Clé Orca → libellé WebSlicer (`Plate.plateType`) ; inconnu → `null`. */
export function bedTypeToPlateType(bedType: string): string | null {
	const bed = bedType === 'SuperTack Plate' ? 'Supertack Plate' : bedType;
	return PLATE_TYPE_BED_TYPES.find(([, b]) => b === bed)?.[0] ?? null;
}

/** Libellé WebSlicer (ou clé Orca) → clé Orca de `curr_bed_type` ; inconnu → `null`. */
export function plateTypeToBedType(plateType: string): string | null {
	return PLATE_TYPE_BED_TYPES.find(([l, b]) => l === plateType || b === plateType)?.[1] ?? null;
}

/** Type de plaque (libellé `PLATE_TYPES`) d'un nouveau plateau pour cette imprimante. */
export function printerDefaultPlateType(machine: Record<string, unknown>): string {
	return bedTypeToPlateType(printerDefaultBedType(machine)) ?? PLATE_TYPES[0].label;
}
