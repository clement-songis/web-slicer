// Focus des dialogues modaux : à l'ouverture, le focus passe dans le dialogue
// (sinon il reste sur le bouton d'ouverture, hors du modal, et les raccourcis
// d'éditeur — Suppr, R, Ctrl+A… — continuent d'agir sur la scène derrière le
// dialogue) ; il y reste (Tab / Maj+Tab bouclent dans le dialogue, comme un
// modal ARIA) ; à la fermeture, il revient à l'élément d'origine, comme un
// wxDialog d'Orca rend le focus à sa fenêtre parente. Échap ferme le dialogue
// si `onEscape` est fourni, même quand le focus est sur le conteneur.

type Focusable = {
	focus: (options?: FocusOptions) => void;
	isConnected?: boolean;
	hasAttribute?: (name: string) => boolean;
};

export interface DialogFocusOptions {
	/** Fermeture par Échap (sinon laissée au dialogue). */
	onEscape?: () => void;
}

const FOCUSABLE =
	'a[href], button:not([disabled]), input:not([disabled]):not([type="hidden"]), select:not([disabled]), textarea:not([disabled]), summary, [tabindex]:not([tabindex="-1"]), [contenteditable="true"]';

/** Pile des dialogues ouverts : seul le dernier capture Tab / Échap. */
const stack: HTMLElement[] = [];

/** Éléments focalisables visibles d'un conteneur, dans l'ordre du document. */
export function focusables(root: HTMLElement): HTMLElement[] {
	return Array.from(root.querySelectorAll<HTMLElement>(FOCUSABLE)).filter((el) => {
		if (el.closest('[inert], [aria-hidden="true"]')) return false;
		const r = el.getBoundingClientRect?.();
		return !r || r.width > 0 || r.height > 0;
	});
}

/** Action Svelte : `<div role="dialog" tabindex="-1" use:dialogFocus>`. */
export function dialogFocus(
	node: HTMLElement,
	options: DialogFocusOptions | (() => void) = {}
): { update: (o: DialogFocusOptions | (() => void)) => void; destroy: () => void } {
	let opts: DialogFocusOptions = typeof options === 'function' ? { onEscape: options } : options;
	const active = (typeof document !== 'undefined' ? document.activeElement : null) as
		(Element & Focusable) | null;
	const previous = active && typeof active.focus === 'function' ? active : null;
	// Ne vole pas le focus d'un champ déjà focalisé dans le dialogue (autofocus).
	if (!active || !node.contains(active)) node.focus({ preventScroll: true });
	stack.push(node);

	function onKeydown(e: KeyboardEvent) {
		if (stack[stack.length - 1] !== node) return;
		if (e.key === 'Escape' && opts.onEscape) {
			e.preventDefault();
			e.stopPropagation();
			opts.onEscape();
			return;
		}
		if (e.key !== 'Tab' || e.ctrlKey || e.altKey || e.metaKey) return;
		const items = focusables(node);
		const current = document.activeElement as HTMLElement | null;
		if (items.length === 0) {
			e.preventDefault();
			node.focus({ preventScroll: true });
			return;
		}
		const first = items[0];
		const last = items[items.length - 1];
		const inside = !!current && node.contains(current);
		if (!inside || (e.shiftKey && (current === first || current === node))) {
			e.preventDefault();
			e.stopPropagation();
			(e.shiftKey ? last : first).focus();
		} else if (!e.shiftKey && current === last) {
			e.preventDefault();
			e.stopPropagation();
			first.focus();
		}
	}
	const doc = typeof document !== 'undefined' ? document : null;
	doc?.addEventListener?.('keydown', onKeydown, true);

	return {
		update(o) {
			opts = typeof o === 'function' ? { onEscape: o } : (o ?? {});
		},
		destroy() {
			doc?.removeEventListener?.('keydown', onKeydown, true);
			const at = stack.lastIndexOf(node);
			if (at >= 0) stack.splice(at, 1);
			if (previous && previous.isConnected !== false && !previous.hasAttribute?.('disabled')) {
				previous.focus({ preventScroll: true });
			}
		}
	};
}
