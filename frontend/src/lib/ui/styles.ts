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

const BUTTON_BASE =
	'inline-flex items-center justify-center gap-1.5 rounded-md font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:opacity-50 disabled:pointer-events-none';

const BUTTON_SIZE: Record<ButtonSize, string> = {
	sm: 'px-2.5 py-1 text-sm',
	md: 'px-4 py-2 text-sm'
};

const BUTTON_VARIANT: Record<ButtonVariant, string> = {
	primary: 'bg-primary text-primary-content hover:bg-primary-hover',
	secondary: 'border border-border-strong bg-surface-raised text-content hover:bg-overlay',
	ghost: 'text-content-muted hover:bg-overlay hover:text-content',
	danger: 'bg-danger text-white hover:opacity-90'
};

/** Classes d'un bouton selon sa variante et sa taille. */
export function buttonClasses(variant: ButtonVariant = 'primary', size: ButtonSize = 'md'): string {
	return cx(BUTTON_BASE, BUTTON_SIZE[size], BUTTON_VARIANT[variant]);
}

/** Classes d'un bouton-icône (carré) : même palette, padding uniforme. */
export function iconButtonClasses(
	variant: ButtonVariant = 'ghost',
	size: ButtonSize = 'md'
): string {
	const pad = size === 'sm' ? 'p-1' : 'p-1.5';
	return cx(BUTTON_BASE, pad, BUTTON_VARIANT[variant]);
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

// --- Segmented control -------------------------------------------------------

/** Classe d'un segment (option) : actif = primaire, sinon discret. */
export function segmentClasses(active: boolean): string {
	return cx(
		'px-3 py-1 text-sm transition-colors',
		active ? 'bg-primary text-primary-content' : 'text-content-muted hover:bg-overlay'
	);
}

// --- Tabs --------------------------------------------------------------------

/** Classe d'un onglet : actif = souligné accentué, sinon discret. */
export function tabClasses(active: boolean): string {
	return cx(
		'flex-1 px-3 py-2 text-sm transition-colors',
		active
			? 'border-b-2 border-primary font-medium text-primary'
			: 'text-content-muted hover:text-content'
	);
}
