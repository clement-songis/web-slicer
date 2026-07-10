// Codec de peinture au triangle (T056), fidèle au format d'OrcaSlicer
// (`TriangleSelector::serialize` / `FacetsAnnotation::get_triangle_as_string`).
//
// Chaque triangle peint est encodé en un flux de bits regroupés en quartets
// (4 bits) convertis en chiffres hexadécimaux, **insérés en tête** (ordre
// inverse). Pour un triangle non subdivisé (peinture au triangle entier) :
//   - 2 bits « nombre de côtés scindés » = 00 (feuille)
//   - état : si < 3, 2 bits de l'état ; sinon « 11 » puis 4 bits de (état − 3).
// Ainsi l'enforcer (état 1) → "4", le blocker (état 2) → "8" (valeurs Orca).
//
// La subdivision sous-triangle (peinture plus fine que la facette) est un
// raffinement moteur : `decodeFacet` renvoie -1 pour un triangle scindé.

/** États de peinture communs (supports/seam/fuzzy). */
export const ENFORCER = 1;
export const BLOCKER = 2;

function hexDigit(code: number): string {
	return code < 10 ? String(code) : String.fromCharCode(code - 10 + 65);
}

/** Encode l'état (>0) d'un triangle entier en chaîne hexadécimale Orca. */
export function encodeFacet(state: number): string {
	if (state <= 0) return '';
	const bits: number[] = [0, 0]; // nombre de côtés scindés = 0 (feuille)
	if (state < 3) {
		bits.push(state & 1, (state >> 1) & 1);
	} else {
		bits.push(1, 1);
		const m = state - 3;
		for (let i = 0; i < 4; i++) bits.push((m >> i) & 1);
	}
	let out = '';
	for (let o = 0; o < bits.length; o += 4) {
		let code = 0;
		for (let i = 3; i >= 0; i--) code = (code << 1) | (bits[o + i] ?? 0);
		out = hexDigit(code) + out;
	}
	return out;
}

/**
 * Décode une chaîne hexadécimale Orca en état de triangle entier. Renvoie 0 si
 * vide, -1 si le triangle est subdivisé (non géré à la granularité facette).
 */
export function decodeFacet(str: string): number {
	if (str === '') return 0;
	const bits: number[] = [];
	// La chaîne est bâtie en insérant les chiffres en tête → on la renverse pour
	// retrouver l'ordre des quartets (offset 0 en premier).
	for (const ch of str.split('').reverse()) {
		const code = parseInt(ch, 16);
		for (let i = 0; i < 4; i++) bits.push((code >> i) & 1);
	}
	const split = bits[0] | (bits[1] << 1);
	if (split !== 0) return -1; // sous-triangles non pris en charge
	if (bits[2] === 1 && bits[3] === 1) {
		let m = 0;
		for (let i = 0; i < 4; i++) m |= bits[4 + i] << i;
		return m + 3;
	}
	return bits[2] | (bits[3] << 1);
}
