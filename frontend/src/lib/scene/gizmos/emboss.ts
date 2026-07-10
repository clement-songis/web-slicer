// Texte/SVG en relief (T057, gizmos Emboss & SVG) : paramètres de création de
// volume en relief. La génération du maillage (rastérisation de police, tracé
// SVG → contours → extrusion) est une opération moteur (libslic3r/FFI, phase
// P5) ; ici on modélise et on valide les paramètres, purs et testables.

/** Source d'un volume en relief. */
export type EmbossSource = 'text' | 'svg';

/** Paramètres d'un volume en relief (texte ou SVG). */
export interface EmbossParams {
	source: EmbossSource;
	/** Texte à graver (source `text`). */
	text: string;
	/** Contenu SVG (source `svg`). */
	svg: string;
	/** Hauteur de police (mm, source `text`). */
	fontSize: number;
	/** Profondeur d'extrusion (mm). */
	depth: number;
	/** Grave en creux plutôt qu'en relief. */
	embossed: boolean;
}

export function defaultEmbossParams(): EmbossParams {
	return {
		source: 'text',
		text: 'Texte',
		svg: '',
		fontSize: 10,
		depth: 2,
		embossed: true
	};
}

/**
 * Valide les paramètres : profondeur > 0, et contenu non vide selon la source.
 * Renvoie `null` si valide, sinon un message d'erreur.
 */
export function validateEmboss(p: EmbossParams): string | null {
	if (!(p.depth > 0)) return 'la profondeur doit être positive';
	if (p.source === 'text') {
		if (p.text.trim() === '') return 'le texte est vide';
		if (!(p.fontSize > 0)) return 'la taille de police doit être positive';
	} else if (p.svg.trim() === '') {
		return 'le contenu SVG est vide';
	}
	return null;
}
