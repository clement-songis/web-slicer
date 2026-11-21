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
import { m } from '$lib/paraglide/messages';
import {
	baseLocale,
	getLocale,
	locales,
	setLocale as setParaglideLocale
} from '$lib/paraglide/runtime';

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

/** Signature d'une fonction de message Paraglide (nos messages sont sans paramètre). */
type MessageFn = (inputs?: Record<string, never>, options?: { locale?: Locale }) => string;

/**
 * Index inverse : libellé anglais (clé de parité) → fonction de message. Construit
 * une fois en appelant chaque message en `baseLocale` (anglais). Robuste aux
 * identifiants générés (ex. « Import » → export `"import"`) : on mappe par sortie.
 */
const byEnglish = new Map<string, MessageFn>();
for (const value of Object.values(m)) {
	if (typeof value === 'function') {
		const fn = value as MessageFn;
		byEnglish.set(fn({}, { locale: baseLocale }), fn);
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
	key: string
): string {
	const fn = byEnglish.get(key);
	if (fn) return fn({}, { locale: loc });
	return dicts[loc]?.[key] ?? key;
}

/** Vrai si le libellé anglais dispose d'une traduction (Paraglide ou dictionnaire chargé). */
export function isTranslatable(label: string): boolean {
	if (byEnglish.has(label)) return true;
	const dicts = get(lazyDicts);
	return Object.values(dicts).some((d) => d !== undefined && label in d);
}

/** Traduit une clé (libellé anglais) pour une locale ; repli additif sur la clé. */
export function translate(loc: Locale, key: string): string {
	return lookup(get(lazyDicts), loc, key);
}

/** Locale courante (store réactif) ; synchronisée avec Paraglide. */
export const locale = writable<Locale>(currentLocale());

/**
 * Helper réactif : `$t('New Project')`. Se réévalue au changement de locale et
 * à l'arrivée d'un dictionnaire paresseux.
 */
export const t = derived(
	[locale, lazyDicts],
	([loc, dicts]) =>
		(key: string) =>
			lookup(dicts, loc, key)
);

/** Traduit avec la locale courante hors composant (usage impératif). */
export function tr(key: string): string {
	return translate(get(locale), key);
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
