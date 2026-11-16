// Orchestrateur pur de dispatch clavier de l'éditeur (T114) : traduit une
// combinaison de touches en identifiant d'action (mêmes ids que les menus/outils
// câblés en T107–T113). Découplé du DOM → testable sous bun. Isolation par
// contexte : quand le focus est dans un champ éditable, aucun raccourci n'est
// capté (le champ garde toutes ses touches). Les touches réservées au navigateur
// (Ctrl+N, Ctrl+Tab, Ctrl+M) ne sont volontairement pas mappées (exclusions.md).
// F5 est intercepté dans l'éditeur pour recharger le G-code ouvert, comme Orca.
//
// Exception unique : Ctrl/Cmd+S (sauvegarde) n'est PAS résolu ici. Il est la
// propriété exclusive de `SaveControls.svelte` (via `isSaveChord`), qui garde
// l'état de sauvegarde (anti-double envoi, retour visuel) et intercepte la
// touche même depuis un champ éditable (sinon « Enregistrer la page » du
// navigateur). Le mapper aussi ici déclenchait deux sauvegardes concurrentes.

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
		' ': 'space',
		arrowup: 'up',
		arrowdown: 'down',
		arrowleft: 'left',
		arrowright: 'right'
	};
	return special[k] ?? k;
}

/** Symbole imprimable (« ? », « + »…) : Maj fait partie de sa saisie selon la
 *  disposition (« ? » = Maj+, sur AZERTY/QWERTY) et ne doit pas changer l'accord. */
function isShiftedSymbol(key: string): boolean {
	return key.length === 1 && !/[a-z0-9 ]/i.test(key);
}

/** Chaîne canonique d'un accord : modificateurs triés + touche. Cmd(meta)≡Ctrl. */
export function chordString(c: KeyChord): string {
	const parts: string[] = [];
	if (c.ctrl || c.meta) parts.push('ctrl');
	if (c.alt) parts.push('alt');
	if (c.shift && !isShiftedSymbol(c.key)) parts.push('shift');
	parts.push(normKey(c.key));
	return parts.join('+');
}

/** Accord de sauvegarde, propriété exclusive de `SaveControls` (voir en-tête). */
export const SAVE_CHORD = 'ctrl+s';

/** Vrai pour Ctrl/Cmd+S exactement (pas Ctrl+Shift+S = « Enregistrer sous »). */
export function isSaveChord(chord: KeyChord): boolean {
	return chordString(chord) === SAVE_CHORD;
}

/** Accords avec modificateur (ou touches spéciales) → id d'action de menu/app. */
const MODIFIER_CHORDS: Record<string, string> = {
	// `ctrl+s` volontairement absent : géré par SaveControls (isSaveChord).
	'ctrl+shift+s': 'project.saveAs',
	'ctrl+o': 'project.open',
	'ctrl+i': 'import.geometry',
	// Affichés dans le menu Fichier : interceptés pour ne pas laisser le
	// navigateur ouvrir « Rechercher le suivant » / « Imprimer ».
	'ctrl+g': 'export.plate',
	'ctrl+shift+g': 'export.print',
	'ctrl+p': 'app.preferences',
	'ctrl+z': 'edit.undo',
	'ctrl+y': 'edit.redo',
	// Orca accepte aussi Ctrl+Maj+Z pour rétablir.
	'ctrl+shift+z': 'edit.redo',
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
	// Ctrl+R (Slice plate) : intercepté avant le rechargement du navigateur.
	'ctrl+r': 'slice.plate',
	// Tab : bascule Préparer ↔ Aperçu (l'Aperçu a son propre gestionnaire).
	tab: 'view.switchTab',
	// Maj+Tab : replier/déplier le panneau latéral (groupe Plater).
	'shift+tab': 'view.toggleSidebar',
	del: 'edit.deleteSelected',
	backspace: 'edit.deleteSelected',
	esc: 'edit.deselectAll',
	f5: 'project.reload',
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
	// Caméra et drapeau imprimable de l'objet sélectionné (groupe Plater).
	i: 'view.zoomIn',
	o: 'view.zoomOut',
	v: 'object.togglePrintable',
	// Plateau (groupe Plater).
	a: 'arrange',
	q: 'orient',
	// Maj+A : arrange le plateau sélectionné (= plateau actif ici) ;
	// Maj+Q : oriente tous les objets du plateau actif.
	'shift+a': 'arrange',
	'shift+q': 'plate.orientAll',
	// Groupe Objects List : Espace renomme l'objet sélectionné.
	space: 'object.renameSelected',
	// 1–9 : filament (extrudeur) de l'objet/pièce sélectionné.
	...Object.fromEntries(
		Array.from({ length: 9 }, (_, i) => [String(i + 1), `object.extruder.${i + 1}`])
	)
};

/** Extrudeur visé par une action `object.extruder.N` (1–9), sinon `null`. */
export function extruderFromAction(action: string): number | null {
	const m = /^object\.extruder\.([1-9])$/.exec(action);
	return m ? Number(m[1]) : null;
}

/** Pas de déplacement aux flèches (groupe Plater d'Orca) : 10 mm, 1 mm avec Maj. */
export const NUDGE_STEP_MM = 10;
export const NUDGE_FINE_STEP_MM = 1;

/**
 * Flèches du plateau → déplacement XY (mm) de la sélection, ou `null`.
 * Haut/Bas = ±Y, Gauche/Droite = ∓X ; Maj réduit le pas à 1 mm. Ctrl+flèche
 * (déplacement dans l'espace caméra) et Alt ne sont pas des déplacements plateau.
 */
export function resolveNudge(chord: KeyChord, editable: boolean): [number, number] | null {
	if (editable || chord.ctrl || chord.meta || chord.alt) return null;
	const step = chord.shift ? NUDGE_FINE_STEP_MM : NUDGE_STEP_MM;
	switch (normKey(chord.key)) {
		case 'up':
			return [0, step];
		case 'down':
			return [0, -step];
		case 'left':
			return [-step, 0];
		case 'right':
			return [step, 0];
		default:
			return null;
	}
}

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
/**
 * Raccourci simple (touche du canvas, sans modificateur) d'une action, au format
 * d'affichage d'Orca (« M », « R »…), ou null. Sert aux info-bulles « Nom (M) »
 * du rail d'outils et de la barre de plateau : seule une touche réellement
 * câblée dans CANVAS_CHORDS est annoncée.
 */
export function shortcutKeyFor(action: string): string | null {
	for (const [chord, act] of Object.entries(CANVAS_CHORDS)) {
		if (act === action && /^[a-z0-9]$/.test(chord)) return chord.toUpperCase();
	}
	return null;
}

/** Libellé d'info-bulle « Nom (Touche) » (nom seul sans raccourci). */
export function withShortcut(label: string, key: string | null): string {
	return key ? `${label} (${key})` : label;
}

export function isToolAction(action: string): boolean {
	return !action.includes('.') && action !== 'arrange' && action !== 'orient';
}
