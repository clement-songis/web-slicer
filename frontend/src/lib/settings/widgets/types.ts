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
}

/** Classe Tailwind commune des champs de saisie. */
export const FIELD_CLASS =
	'w-full rounded border border-gray-300 px-2 py-1 text-sm ' +
	'disabled:opacity-50 dark:border-gray-600 dark:bg-gray-700 dark:text-gray-100';
