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
	/** Police (id serveur, cf. GET /api/emboss/fonts). */
	font: string;
	/** Graisse (mm, FontProp::boldness d'Orca) ; 0 = police telle quelle. */
	boldness: number;
	/** Largeur de la forme SVG (mm). */
	svgWidth: number;
	/** En relief avec un objet sélectionné : pièce posée sur sa face supérieure
	 *  (comportement du gizmo Emboss d'Orca) plutôt qu'un nouvel objet. */
	onObject?: boolean;
	/** Nom du fichier SVG d'origine (3D/<nom>.svg du 3MF), conservé à la ré-édition. */
	svgFileName?: string;
}

/**
 * Placement d'un relief ajouté comme pièce d'un objet (phase 26) : centré en
 * XY sur l'objet, posé sur sa face supérieure — repère recentré de l'objet.
 * `objectMaxZ` : Z max du maillage recentré de l'objet ; `size` : emprise du
 * maillage du relief (mm). Renvoie le décalage du centre de la pièce.
 */
export function embossPartOffset(
	objectMaxZ: number,
	size: readonly [number, number, number]
): [number, number, number] {
	const r = (v: number) => Math.round(v * 10000) / 10000;
	return [0, 0, r(objectMaxZ + size[2] / 2)];
}

export function defaultEmbossParams(): EmbossParams {
	return {
		source: 'text',
		text: 'Texte',
		svg: '',
		font: 'noto-sans',
		boldness: 0,
		svgWidth: 30,
		fontSize: 10,
		depth: 2,
		embossed: true,
		onObject: true
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

/**
 * Phase 28 — paramètres ré-éditables renvoyés par /emboss et /emboss-svg
 * (parts[].emboss, recopiés dans objects[].emboss / volumes[].emboss) →
 * paramètres de l'outil. null si le JSON n'est ni texte ni SVG.
 */
export function paramsFromEmboss(e: unknown): EmbossParams | null {
	if (!e || typeof e !== 'object') return null;
	const o = e as Record<string, unknown>;
	const num = (v: unknown, d: number) => (typeof v === 'number' && Number.isFinite(v) ? v : d);
	const str = (v: unknown, d: string) => (typeof v === 'string' ? v : d);
	const base = defaultEmbossParams();
	if (o.type === 'text' && typeof o.text === 'string')
		return {
			...base,
			source: 'text',
			text: o.text,
			font: str(o.font, base.font),
			fontSize: num(o.size_mm, base.fontSize),
			depth: num(o.depth_mm, base.depth),
			boldness: num(o.boldness_mm, 0)
		};
	if (o.type === 'svg' && typeof o.svg === 'string')
		return {
			...base,
			source: 'svg',
			svg: o.svg,
			svgWidth: num(o.width_mm, base.svgWidth),
			depth: num(o.depth_mm, base.depth),
			svgFileName: str(o.file_name, '') || undefined
		};
	return null;
}

/** Libellé du menu (Orca : « Edit text » / « Edit SVG »). */
export function embossEditLabel(e: unknown): string | null {
	const p = paramsFromEmboss(e);
	return p ? (p.source === 'svg' ? 'Modifier le SVG' : 'Modifier le texte') : null;
}
