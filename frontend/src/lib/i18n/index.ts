// i18n **additif** fr/en (T080, FR-072). Principe : les libellés de parité
// anglais d'OrcaSlicer sont les **clés**. `translate` renvoie la traduction si
// elle existe pour la locale, sinon la clé anglaise elle-même — la parité prime,
// la traduction est un supplément. Sans dépendance externe : un store de locale
// + un helper réactif `t`.

import { derived, get, writable } from 'svelte/store';
import { fr } from './fr';

/** Locales prises en charge. `en` = identité (clés = libellés de parité). */
export type Locale = 'en' | 'fr';

export const LOCALES: readonly Locale[] = ['en', 'fr'] as const;

/** Dictionnaires par locale. `en` est vide : la clé est déjà l'anglais. */
export const DICTIONARIES: Record<Locale, Record<string, string>> = {
	en: {},
	fr
};

const STORAGE_KEY = 'web-slicer.locale';

/** Locale courante (store réactif). Défaut `en` (parité). */
export const locale = writable<Locale>('en');

/** Traduit une clé pour une locale ; repli additif sur la clé (anglais). */
export function translate(loc: Locale, key: string): string {
	return DICTIONARIES[loc][key] ?? key;
}

/**
 * Helper réactif : `$t('New Project')`. Se réévalue au changement de locale.
 */
export const t = derived(locale, (loc) => (key: string) => translate(loc, key));

/** Traduit avec la locale courante hors composant (usage impératif). */
export function tr(key: string): string {
	return translate(get(locale), key);
}

/** Change la locale et la mémorise (si le stockage local est disponible). */
export function setLocale(next: Locale): void {
	locale.set(next);
	try {
		localStorage.setItem(STORAGE_KEY, next);
	} catch {
		// Stockage indisponible (SSR, mode privé) : ignoré.
	}
}

/**
 * Initialise la locale au démarrage (navigateur) : préférence mémorisée, sinon
 * langue du navigateur (`fr*` → français), sinon anglais. Sans effet côté SSR.
 */
export function initLocale(): void {
	if (typeof window === 'undefined') return;
	let chosen: Locale = 'en';
	try {
		const saved = localStorage.getItem(STORAGE_KEY);
		if (saved === 'en' || saved === 'fr') {
			chosen = saved;
		} else if (navigator.language?.toLowerCase().startsWith('fr')) {
			chosen = 'fr';
		}
	} catch {
		// Accès stockage/navigator indisponible : anglais par défaut.
	}
	locale.set(chosen);
}
