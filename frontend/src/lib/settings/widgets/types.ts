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

/**
 * Unité affichée à droite d'un champ. Le registre généré colle parfois
 * l'infobulle à l'unité (12 clés, ex. top_shell_layers : « layersThis is the
 * number… ») : on ne garde que l'unité (avant la première phrase).
 */
export function unitText(sidetext: string | null | undefined): string {
	const s = (sidetext ?? '').trim();
	if (s.length <= 12) return s;
	// Unité connue en tête (ex. « mm/sperimeter_feed_rate », clé héritée collée).
	const known = /^(mm\/s²|mm\/s|mm³\/s|mm³|mm|°C|%|layers?|s)(?=[a-z_A-Z])/.exec(s);
	if (known) return known[1];
	const m = /^(.*?)(?=[A-Z][a-z])/.exec(s);
	return m && m[1].trim() ? m[1].trim() : '';
}

/**
 * Phase 28 — séparateur décimal homogène : les champs numériques natifs
 * (input type=number) affichent celui de la langue du navigateur (« 0,2 » en
 * français) ; les champs texte « mm ou % » / « % » affichent désormais le même.
 * La valeur stockée garde le point (format de config d'Orca).
 */
export function decimalSeparator(locale?: string): string {
	try {
		const lang =
			locale ?? (typeof navigator !== 'undefined' ? navigator.language : undefined) ?? 'en';
		return (1.5).toLocaleString(lang).includes(',') ? ',' : '.';
	} catch {
		return '.';
	}
}

/** Valeur de config → texte affiché (point → séparateur local entre deux chiffres). */
export function toDisplayDecimal(value: unknown, sep = decimalSeparator()): string {
	const s = value === undefined || value === null ? '' : String(value);
	return sep === '.' ? s : s.replace(/(\d)\.(\d)/g, '$1' + sep + '$2');
}

/** Texte saisi → valeur de config (virgule décimale → point). */
export function fromDisplayDecimal(text: string): string {
	return text.replace(/(\d),(\d)/g, '$1.$2').replace(/^,(\d)/, '0.$1');
}
