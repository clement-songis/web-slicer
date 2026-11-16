// Contrat commun des widgets de réglage (T041). Chaque contrôle reçoit la
// définition du paramètre (bornes, enums, unité…) et une valeur liable ;
// l'étiquette et l'infobulle sont posées par `OptionLine` (T042).

import type { ParamDef } from '../../../generated/params';

/** Props partagées par tous les widgets d'édition. `value` est `$bindable`. */
export interface WidgetProps<T> {
	/** Définition registre du paramètre édité. */
	def: ParamDef;
	/** Valeur courante (liaison bidirectionnelle). */
	value: T;
	/** Champ verrouillé (preset système, hérité non surchargé…). */
	disabled?: boolean;
	/** Indice du contrôle dans une valeur vectorielle (contrat UI). */
	index?: number;
}

/** Classe Tailwind commune des champs de saisie (jetons du design system). */
export const FIELD_CLASS =
	'w-full rounded border border-border-strong bg-surface-raised px-2 py-1 text-sm ' +
	'text-content disabled:opacity-50';

/** Forme Orca d'un pourcentage : nombre → « 15% », chaîne telle quelle (vide si absent). */
export function percentText(value: unknown): string {
	if (typeof value === 'number' && Number.isFinite(value)) return `${value}%`;
	return value === undefined || value === null ? '' : String(value);
}

/**
 * Enum « ouvert » : combo éditable chez Orca (`*_enum_open`, `select_open`) ou
 * enum dont le registre n'a pas extrait les valeurs (liste vide) — rendu en
 * champ texte avec suggestions plutôt qu'en `<select>` fermé.
 */
export function isOpenEnum(def: { guiType: string; enumValues: readonly string[] }): boolean {
	return (
		def.guiType.endsWith('_enum_open') ||
		def.guiType === 'select_open' ||
		def.enumValues.length === 0
	);
}
