// Validation d'une valeur de paramètre contre le registre (T043) : bornes
// numériques et appartenance aux enums, mêmes garde-fous qu'OrcaSlicer.
// Pur et sans dépendance Svelte → testable isolément.

import type { ParamDef } from '../../generated/params';
import { resolveWidgetKind } from './widgets/resolve';

/** Résultat de validation : `ok`, plus un message en cas d'échec. */
export interface Validation {
	ok: boolean;
	message?: string;
}

const OK: Validation = { ok: true };

/** Extrait la part numérique d'un FloatOrPercent (« 40% » → 40, « 0.4 » → 0.4). */
function numericPart(value: unknown): number | null {
	if (typeof value === 'number') return Number.isFinite(value) ? value : null;
	if (typeof value === 'string') {
		const trimmed = value.trim().replace(/%$/, '');
		const n = Number(trimmed);
		return trimmed !== '' && Number.isFinite(n) ? n : null;
	}
	return null;
}

/** Vérifie les bornes min/max présentes dans la définition. */
function checkBounds(def: ParamDef, n: number): Validation {
	if (def.min !== null && n < def.min) {
		return { ok: false, message: `valeur minimale : ${def.min}` };
	}
	if (def.max !== null && n > def.max) {
		return { ok: false, message: `valeur maximale : ${def.max}` };
	}
	return OK;
}

/**
 * Valide une valeur pour un paramètre. Les types non contraints (chaîne,
 * multiligne, couleur…) passent toujours ; les enums fermés exigent une valeur
 * connue, les numériques respectent les bornes.
 */
export function validateValue(def: ParamDef, value: unknown): Validation {
	switch (resolveWidgetKind(def)) {
		case 'bool':
			return typeof value === 'boolean' ? OK : { ok: false, message: 'booléen attendu' };

		case 'int': {
			const n = numericPart(value);
			if (n === null) return { ok: false, message: 'nombre attendu' };
			if (!Number.isInteger(n)) return { ok: false, message: 'entier attendu' };
			return checkBounds(def, n);
		}

		case 'float':
		case 'percent':
		case 'floatOrPercent': {
			const n = numericPart(value);
			if (n === null) return { ok: false, message: 'nombre attendu' };
			return checkBounds(def, n);
		}

		case 'enum': {
			// Enum fermé (coEnum*) : la valeur doit appartenir au registre.
			// Enum ouvert (numérique + suggestions) : saisie libre tolérée.
			if (def.type.startsWith('coEnum')) {
				return typeof value === 'string' && def.enumValues.includes(value)
					? OK
					: { ok: false, message: 'valeur d’enum inconnue' };
			}
			return OK;
		}

		case 'point': {
			return Array.isArray(value) && value.every((v) => typeof v === 'number')
				? OK
				: { ok: false, message: 'coordonnées numériques attendues' };
		}

		default:
			return OK;
	}
}
