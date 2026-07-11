// Orchestrateur pur de dispatch clavier de l'éditeur (T114) : traduit une
// combinaison de touches en identifiant d'action (mêmes ids que les menus/outils
// câblés en T107–T113). Découplé du DOM → testable sous bun. Isolation par
// contexte : quand le focus est dans un champ éditable, aucun raccourci n'est
// capté (le champ garde toutes ses touches). Les touches réservées au navigateur
// (Ctrl+N, F5, Ctrl+Tab, Ctrl+M) ne sont volontairement pas mappées (exclusions.md).

/** Accord clavier neutre (indépendant du DOM). */
export interface KeyChord {
	key: string;
	ctrl?: boolean;
	shift?: boolean;
	alt?: boolean;
	meta?: boolean;
}

/** Normalise une touche spéciale vers un jeton stable. */
function normKey(key: string): string {
	const k = key.toLowerCase();
	const special: Record<string, string> = {
		delete: 'del',
		backspace: 'backspace',
		escape: 'esc',
		' ': 'space'
	};
	return special[k] ?? k;
}

/** Chaîne canonique d'un accord : modificateurs triés + touche. Cmd(meta)≡Ctrl. */
export function chordString(c: KeyChord): string {
	const parts: string[] = [];
	if (c.ctrl || c.meta) parts.push('ctrl');
	if (c.alt) parts.push('alt');
	if (c.shift) parts.push('shift');
	parts.push(normKey(c.key));
	return parts.join('+');
}

/** Accords avec modificateur (ou touches spéciales) → id d'action de menu/app. */
const MODIFIER_CHORDS: Record<string, string> = {
	'ctrl+s': 'project.save',
	'ctrl+shift+s': 'project.saveAs',
	'ctrl+o': 'project.open',
	'ctrl+i': 'import.geometry',
	'ctrl+z': 'edit.undo',
	'ctrl+y': 'edit.redo',
	'ctrl+c': 'edit.copy',
	'ctrl+x': 'edit.cut',
	'ctrl+v': 'edit.paste',
	'ctrl+a': 'edit.selectAll',
	'ctrl+d': 'edit.deleteAll',
	'ctrl+k': 'edit.clone',
	'ctrl+0': 'view.default',
	'ctrl+1': 'view.top',
	'ctrl+2': 'view.bottom',
	'ctrl+3': 'view.front',
	'ctrl+4': 'view.rear',
	'ctrl+5': 'view.left',
	'ctrl+6': 'view.right',
	del: 'edit.deleteSelected',
	backspace: 'edit.deleteSelected',
	esc: 'edit.deselectAll',
	'?': 'help.shortcuts'
};

/** Touches simples (sans modificateur) actives uniquement sur le canvas (non éditable). */
const CANVAS_CHORDS: Record<string, string> = {
	// Gizmos (groupe Gizmo, Annexe B §B.6).
	m: 'move',
	r: 'rotate',
	s: 'scale',
	f: 'flatten',
	c: 'cut',
	b: 'boolean',
	h: 'fuzzy-paint',
	p: 'seam-paint',
	t: 'emboss',
	u: 'measure',
	y: 'assembly',
	e: 'brim-ears',
	// Plateau (groupe Plater).
	a: 'arrange',
	q: 'orient'
};

/**
 * Résout un accord en identifiant d'action, ou `null`. `editable=true` (focus
 * dans un champ) désactive tous les raccourcis — isolation par contexte.
 */
export function resolveShortcut(chord: KeyChord, editable: boolean): string | null {
	if (editable) return null;
	const combo = chordString(chord);
	return MODIFIER_CHORDS[combo] ?? CANVAS_CHORDS[combo] ?? null;
}

/** Vrai si l'action est un outil du rail (sinon c'est une action de menu/app). */
export function isToolAction(action: string): boolean {
	return !action.includes('.') && action !== 'arrange' && action !== 'orient';
}
