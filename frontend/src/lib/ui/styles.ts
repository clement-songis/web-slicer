// Constructeurs de classes des primitives UI (T094) — logique pure, testable
// sous bun (le repo n'a pas de harnais de rendu Svelte : la logique vit ici, le
// `.svelte` reste présentationnel). Toutes les classes s'appuient EXCLUSIVEMENT
// sur les jetons du système de design (T093), jamais sur une échelle brute.

/** Joint des fragments de classes en ignorant les vides (helper interne). */
export function cx(...parts: (string | false | null | undefined)[]): string {
	return parts.filter(Boolean).join(' ');
}

// --- Button ------------------------------------------------------------------

export type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'danger';
export type ButtonSize = 'sm' | 'md';

/** Anneau de focus clavier commun à toutes les primitives (WCAG 2.4.7). */
export const FOCUS_RING =
	'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent focus-visible:ring-offset-1 focus-visible:ring-offset-surface-raised';

/** Anneau intérieur pour les éléments accolés (segments, onglets). */
export const FOCUS_RING_INSET =
	'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-accent';

// Les recettes visuelles vivent dans `routes/layout.css` (@layer components :
// .ws-btn, .ws-btn-<variante>, .ws-btn-<taille>) : une seule source pour les
// composants Svelte ET le balisage brut des écrans.

export type ButtonSizeFull = ButtonSize | 'lg';

const BUTTON_SIZE: Record<ButtonSizeFull, string> = {
	sm: 'ws-btn-sm',
	md: '',
	lg: 'ws-btn-lg'
};

/** Classes d'un bouton selon sa variante et sa taille (hauteurs 28/32/40 px). */
export function buttonClasses(
	variant: ButtonVariant = 'primary',
	size: ButtonSizeFull = 'md'
): string {
	return cx('ws-btn', BUTTON_SIZE[size], `ws-btn-${variant}`, FOCUS_RING);
}

/** Classes d'un bouton-icône (carré) : même palette, même hauteur que le bouton. */
export function iconButtonClasses(
	variant: ButtonVariant = 'ghost',
	size: ButtonSize = 'md'
): string {
	return cx(
		'ws-icon-btn',
		size === 'sm' && 'ws-icon-btn-sm',
		variant !== 'ghost' && `ws-btn-${variant}`,
		FOCUS_RING
	);
}

/** Classes d'un champ (input/select/textarea) selon la taille. */
export function fieldClasses(size: ButtonSizeFull = 'md'): string {
	return cx('ws-field', size === 'sm' && 'ws-field-sm', size === 'lg' && 'ws-field-lg');
}

// --- Banner ------------------------------------------------------------------

export type BannerTone = 'info' | 'success' | 'warning' | 'danger';

const BANNER_BASE = 'flex items-center justify-between gap-4 rounded-md px-4 py-2 text-sm';

const BANNER_TONE: Record<BannerTone, string> = {
	info: 'border border-border bg-surface-sunken text-content-muted',
	success: 'bg-success-soft text-success-content',
	warning: 'bg-warning-soft text-warning-content',
	danger: 'bg-danger-soft text-danger-content'
};

/** Classes d'un bandeau selon sa tonalité. */
export function bannerClasses(tone: BannerTone = 'info'): string {
	return cx(BANNER_BASE, BANNER_TONE[tone]);
}

// --- Segmented control / Tabs / Outils --------------------------------------
// Recettes .ws-seg-item / .ws-tab / .ws-tool (layout.css) : mêmes classes pour
// les composants et les écrans (modes d'affichage, onglets, rail d'outils).

/** Classe d'un segment (option) : actif = primaire, sinon discret. */
export function segmentClasses(active: boolean): string {
	return cx('ws-seg-item', active && 'ws-seg-on');
}

/** Classe d'un onglet : actif = souligné accentué, sinon discret. */
export function tabClasses(active: boolean): string {
	return cx('ws-tab flex-1', active && 'ws-tab-on');
}

/** Classe d'un bouton d'outil (rail, barre flottante). */
export function toolClasses(active: boolean, size: 'sm' | 'md' = 'md'): string {
	return cx('ws-tool', size === 'sm' && 'ws-tool-sm', active && 'ws-tool-on', FOCUS_RING);
}
