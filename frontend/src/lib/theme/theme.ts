// Thème clair/sombre (T093). Principe : une **préférence** (`light|dark|system`)
// est mémorisée et exposée en store réactif ; elle est **résolue** en un thème
// concret (`light|dark`) posé sur `data-theme` de <html>. `system` suit l'OS via
// `matchMedia`. Le variant `dark:` de Tailwind est donc data-driven (cf.
// `layout.css`) : les composants non migrés restent fonctionnels dans les deux
// thèmes. Sans dépendance externe : un store + des helpers purs testables.

import { get, writable } from 'svelte/store';

/** Préférence utilisateur. `system` délègue au réglage du système. */
export type ThemePref = 'light' | 'dark' | 'system';

/** Thème effectivement appliqué (jamais `system`). */
export type ResolvedTheme = 'light' | 'dark';

export const THEME_PREFS: readonly ThemePref[] = ['light', 'dark', 'system'] as const;

const STORAGE_KEY = 'web-slicer.theme';

/** Préférence courante (store réactif). Défaut `system`. */
export const themePref = writable<ThemePref>('system');

/** Résout une préférence en thème concret ; `system` → OS (`systemDark`). */
export function resolveTheme(pref: ThemePref, systemDark: boolean): ResolvedTheme {
	if (pref === 'light' || pref === 'dark') return pref;
	return systemDark ? 'dark' : 'light';
}

/** Normalise une valeur stockée arbitraire en préférence valide (repli `system`). */
export function parseThemePref(value: unknown): ThemePref {
	return value === 'light' || value === 'dark' || value === 'system' ? value : 'system';
}

/** Vrai si l'OS demande le sombre (faux hors navigateur). */
function systemPrefersDark(): boolean {
	return (
		typeof window !== 'undefined' &&
		typeof window.matchMedia === 'function' &&
		window.matchMedia('(prefers-color-scheme: dark)').matches
	);
}

/** Applique un thème concret sur <html> (`data-theme`). */
function applyResolved(theme: ResolvedTheme): void {
	if (typeof document !== 'undefined') {
		document.documentElement.setAttribute('data-theme', theme);
	}
}

/** Recalcule et applique le thème depuis la préférence courante. */
function reapply(): void {
	applyResolved(resolveTheme(get(themePref), systemPrefersDark()));
}

/** Change la préférence : mémorise, met à jour le store et applique. */
export function setTheme(pref: ThemePref): void {
	themePref.set(pref);
	try {
		localStorage.setItem(STORAGE_KEY, pref);
	} catch {
		// Stockage indisponible (mode privé…) : la session reste cohérente en mémoire.
	}
	reapply();
}

/**
 * Initialise le thème au démarrage : préférence mémorisée puis application. En
 * mode `system`, réagit aux changements de l'OS tant que la préférence le reste.
 */
export function initTheme(): void {
	if (typeof window === 'undefined') return;
	let saved: ThemePref = 'system';
	try {
		saved = parseThemePref(localStorage.getItem(STORAGE_KEY));
	} catch {
		// Accès stockage indisponible : `system` par défaut.
	}
	themePref.set(saved);
	reapply();

	if (typeof window.matchMedia === 'function') {
		window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
			if (get(themePref) === 'system') reapply();
		});
	}
}
