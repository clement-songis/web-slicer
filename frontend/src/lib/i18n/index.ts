// i18n **additif** fr/en (T080, FR-072) — adossé à Paraglide JS (inlang).
// Principe de parité inchangé : les libellés anglais d'OrcaSlicer sont les
// **clés** (source de vérité, tracées dans `menus.ts`/`shortcuts.ts`) ; la
// traduction est un supplément, repli sur l'anglais si absente.
//
// Paraglide compile `messages/{en,fr}.json` en fonctions typées et
// tree-shakées (`src/lib/paraglide/`, généré — jamais édité). Comme le reste de
// l'app adresse les libellés par leur **texte anglais** (données de menu…), on
// construit un index inverse « libellé anglais → fonction de message », puis on
// expose l'API historique (`t`, `tr`, `translate`, `setLocale`, `locale`). Le
// store Svelte garde la réactivité au changement de locale (Paraglide bascule
// sans rechargement via `reload: false`).

import { derived, get, writable } from 'svelte/store';
import { m } from '#lib/paraglide/messages';
import {
	baseLocale,
	getLocale,
	locales,
	setLocale as setParaglideLocale
} from '#lib/paraglide/runtime';

/** Locales prises en charge (dérivées du projet inlang). */
export type Locale = (typeof locales)[number];

export const LOCALES: readonly Locale[] = locales;

/**
 * Nom d'une langue dans cette langue (Orca : « English », « Français » dans
 * Preferences › Language), jamais son code brut.
 */
const LANGUAGE_NAMES: Record<string, string> = { en: 'English', fr: 'Français' };
export function languageName(code: string): string {
	if (LANGUAGE_NAMES[code]) return LANGUAGE_NAMES[code];
	try {
		const name = new Intl.DisplayNames([code], { type: 'language' }).of(code);
		return name ? name.charAt(0).toLocaleUpperCase(code) + name.slice(1) : code;
	} catch {
		return code;
	}
}

/** Signature d'une fonction de message Paraglide (paramètres `{nom}` facultatifs). */
type MessageFn = (inputs?: Record<string, unknown>, options?: { locale?: Locale }) => string;

/** Valeurs d'interpolation d'un message : `$t('Plate {n}', { n: 2 })`. */
export type MessageParams = Record<string, string | number>;

/**
 * Entrées « miroir » : chaque variable `{nom}` se rend `{nom}`. Les messages
 * paramétrés produisent ainsi leur gabarit (« Plate {n} »), qui sert de clé
 * anglaise ; l'interpolation est faite ensuite par `interpolate`.
 */
const TEMPLATE_INPUTS = new Proxy({} as Record<string, unknown>, {
	get: (_target, prop) => (typeof prop === 'string' ? `{${prop}}` : undefined)
});

/**
 * Contexte de désambiguïsation : « Cut||tool » s'affiche « Cut » en anglais et
 * peut avoir une autre traduction que « Cut ». Le suffixe n'est jamais affiché.
 */
function stripContext(text: string): string {
	const i = text.indexOf('||');
	return i < 0 ? text : text.slice(0, i);
}

function interpolate(text: string, params?: MessageParams): string {
	if (!params) return text;
	return text.replace(/\{(\w+)\}/g, (whole, name: string) =>
		name in params ? String(params[name]) : whole
	);
}

/**
 * Index inverse : libellé anglais (clé de parité) → fonction de message. Construit
 * une fois en appelant chaque message en `baseLocale` (anglais). Robuste aux
 * identifiants générés (ex. « Import » → export `"import"`) : on mappe par sortie.
 */
const byEnglish = new Map<string, MessageFn>();
for (const value of Object.values(m)) {
	if (typeof value === 'function') {
		const fn = value as MessageFn;
		byEnglish.set(fn(TEMPLATE_INPUTS, { locale: baseLocale }), fn);
	}
}

/**
 * Dictionnaires chargés à la demande (« libellé anglais → traduction ») : les
 * gros volumes (≈1100 libellés/infobulles des réglages) ne sont pas compilés
 * par Paraglide dans le chunk initial mais importés à l'ouverture du dialogue
 * (`lib/settings/i18n`). Même contrat additif : repli sur la clé anglaise.
 */
const lazyDicts = writable<Partial<Record<Locale, Record<string, string>>>>({});

/** Ajoute un dictionnaire paresseux pour une locale (fusion, réactif). */
export function registerTranslations(loc: Locale, dict: Record<string, string>): void {
	lazyDicts.update((all) => ({ ...all, [loc]: { ...all[loc], ...dict } }));
}

function lookup(
	dicts: Partial<Record<Locale, Record<string, string>>>,
	loc: Locale,
	key: string,
	params?: MessageParams
): string {
	const fn = byEnglish.get(key);
	const text = fn ? fn(TEMPLATE_INPUTS, { locale: loc }) : (dicts[loc]?.[key] ?? key);
	return interpolate(stripContext(text), params);
}

/** Vrai si le libellé anglais dispose d'une traduction (Paraglide ou dictionnaire chargé). */
export function isTranslatable(label: string): boolean {
	if (byEnglish.has(label)) return true;
	const dicts = get(lazyDicts);
	return Object.values(dicts).some((d) => d !== undefined && label in d);
}

/** Traduit une clé (libellé anglais) pour une locale ; repli additif sur la clé. */
export function translate(loc: Locale, key: string, params?: MessageParams): string {
	return lookup(get(lazyDicts), loc, key, params);
}

/** Locale courante (store réactif) ; synchronisée avec Paraglide. */
export const locale = writable<Locale>(currentLocale());

/** A3 (audit 42) : `<html lang>` suit la langue de l'interface (lecteurs
 *  d'écran, césure, guillemets) ; `app.html` ne porte que la valeur SSR. */
export function syncDocumentLang(
	loc: Locale,
	doc: { documentElement?: { lang: string } } | undefined = typeof document === 'undefined'
		? undefined
		: document
): void {
	if (doc?.documentElement && doc.documentElement.lang !== loc) doc.documentElement.lang = loc;
}
locale.subscribe((loc) => syncDocumentLang(loc));

/**
 * Helper réactif : `$t('New Project')`. Se réévalue au changement de locale et
 * à l'arrivée d'un dictionnaire paresseux.
 */
export const t = derived(
	[locale, lazyDicts],
	([loc, dicts]) =>
		(key: string, params?: MessageParams) =>
			lookup(dicts, loc, key, params)
);

/** Traduit avec la locale courante hors composant (usage impératif). */
export function tr(key: string, params?: MessageParams): string {
	return translate(get(locale), key, params);
}

/** Étiquette BCP 47 de la locale (séparateurs décimaux : « 0,26 » en fr, « 0.26 » en en). */
export function numberLocale(loc: Locale = get(locale)): string {
	return loc === 'fr' ? 'fr-FR' : 'en-US';
}

/**
 * Nombre à décimales fixes avec le séparateur de la langue courante (sans
 * séparateur de milliers, comme les champs d'Orca) : fmtNum(14.336, 2) →
 * « 14,34 » (fr) / « 14.34 » (en).
 */
export function fmtNum(value: number, digits = 2, loc: Locale = get(locale)): string {
	if (!Number.isFinite(value)) return String(value);
	return value.toLocaleString(numberLocale(loc), {
		minimumFractionDigits: digits,
		maximumFractionDigits: digits,
		useGrouping: false
	});
}

/**
 * Nombre sans zéros superflus, séparateur de la langue (audit 41 V5) :
 * « 0,32 », « 0,002 », « 1,7 » en français ; « 0.32 » en anglais.
 */
export function fmtDec(value: number, maxDigits = 4, loc: Locale = get(locale)): string {
	if (!Number.isFinite(value)) return String(value);
	return value.toLocaleString(numberLocale(loc), {
		maximumFractionDigits: maxDigits,
		useGrouping: false
	});
}

/** Change la locale (persistée par la stratégie Paraglide) sans recharger la page. */
export function setLocale(next: Locale): void {
	setParaglideLocale(next, { reload: false });
	locale.set(next);
}

/**
 * Initialise la locale au démarrage : Paraglide résout via sa stratégie
 * (localStorage → langue du navigateur → anglais). On synchronise le store.
 */
export function initLocale(): void {
	locale.set(currentLocale());
}

/** Locale Paraglide courante, repli sur l'anglais si non résolue (SSR). */
function currentLocale(): Locale {
	try {
		return getLocale();
	} catch {
		return baseLocale;
	}
}
